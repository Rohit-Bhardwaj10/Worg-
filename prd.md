Tiling WM for Windows: Problem Analysis, Approach and Build Plan
Prepared for Rohit Bhardwaj. Working document, v1.

1. Goal and success criteria
   Build a tiling window manager for Windows that matches the behavior and feel of Hyprland (dynamic dwindle/master tiling, gaps, workspaces, rules, scriptable IPC, hot-reloaded config) while being stable, fast and predictable, which is where existing Windows tilers fall short.
   Success means:
   • Stable: after a week of uptime, the layout still matches what is on screen. No ghost tiles, no stuck windows, no crashes.
   • Fast: event to applied layout in a few milliseconds, no input lag from our hooks, idle CPU near zero.
   • Predictable: the same input always yields the same layout. Anything the WM is unsure about is left alone rather than forced into the tree.
   • Debuggable: any bug report can be turned into a reproducible test.
   Non-goals for v1: blur on third-party windows (not possible on Windows), replacing DWM, a built-in status bar, complex animations.
2. How window management works on Windows
   Understanding this is the foundation for every design decision below.
   2.1 The compositor is not ours
   On Linux with Hyprland, the WM is the compositor: it decides how every pixel is drawn, so animation, blur and rounding are native. On Windows the compositor is DWM (Desktop Window Manager) and it cannot be replaced. Applications draw into their own surfaces, DWM composites them, and our process is just another application.
   So a Windows tiling WM is a window controller. It observes windows through Win32 APIs and asks them to move and resize. It can influence DWM through a limited set of attributes (border color, corner preference, cloaking) but cannot change how DWM draws. This is the ceiling of the platform, and the design must work within it.
   2.2 The window model
   Every top-level window is identified by an HWND. Each has:
   • Styles (GetWindowLongPtr with GWL_STYLE and GWL_EXSTYLE): caption, resizable frame, WS_EX_TOOLWINDOW, WS_EX_APPWINDOW, WS_EX_NOACTIVATE and more. These are the main signals for deciding whether a window should be tiled.
   • Class name and owning process (GetClassName, GetWindowThreadProcessId): used for rules and exceptions.
   • Owner/parent relationships: dialogs are usually owned by a main window and should normally float.
   • Visibility and cloaked state: a window can be "visible" to Win32 yet cloaked by DWM (for example windows on another virtual desktop, or UWP windows that are not really shown). DwmGetWindowAttribute(DWMWA_CLOAKED) tells us.
   • Geometry: GetWindowRect includes invisible resize borders (roughly 7px on the left, right and bottom on Windows 10/11). The visible rectangle is DWMWA_EXTENDED_FRAME_BOUNDS. A tiler must compute the difference per window and compensate, otherwise gaps look uneven and windows overlap.
   2.3 Observing: events
   There is no "window manager protocol" like X11's substructure redirect that lets a WM intercept and veto changes. We only get notifications after the fact, mainly via SetWinEventHook:
   • EVENT_OBJECT_CREATE, EVENT_OBJECT_SHOW, EVENT_OBJECT_HIDE, EVENT_OBJECT_DESTROY
   • EVENT_OBJECT_CLOAKED / EVENT_OBJECT_UNCLOAKED
   • EVENT_SYSTEM_FOREGROUND (focus changes)
   • EVENT_SYSTEM_MINIMIZESTART / MINIMIZEEND
   • EVENT_SYSTEM_MOVESIZESTART / MOVESIZEEND (user dragging)
   • EVENT_OBJECT_LOCATIONCHANGE (very noisy), EVENT_OBJECT_NAMECHANGE
   We register with WINEVENT_OUTOFCONTEXT so callbacks are delivered to our own thread through its message loop and cannot crash other processes. Key facts about these events: they can arrive out of order, in bursts, duplicated, or (rarely) not at all; windows are often created hidden and shown later; and some apps create, destroy and recreate windows during startup. Events are hints, not truth.
   2.4 Acting: moving and focusing windows
   • Positioning: SetWindowPos (or batched BeginDeferWindowPos / DeferWindowPos / EndDeferWindowPos). Calling it on a window owned by a hung process can block our thread unless we pass SWP_ASYNCWINDOWPOS.
   • Focus: SetForegroundWindow is restricted by Windows (the foreground lock). A background process generally cannot steal focus unless it recently received input or uses a workaround. Reliable focus switching needs a tested technique and a fallback.
   • Minimum sizes and fighting apps: some windows enforce a minimum size, some ignore resizing, and some (certain Electron apps, games) resize themselves back. The WM must tolerate a window refusing its assigned rectangle.
   2.5 Input
   • RegisterHotKey: simple and reliable, but some key combinations (notably many Win+ combos) are reserved by the shell.
   • WH_KEYBOARD_LL low-level hook: can capture almost any combination and implement modes/submaps like Hyprland, but the callback runs on our thread in the input path. If it is slow, Windows times it out and silently removes the hook, and system-wide typing lags. The callback must do almost nothing.
   • Mouse: WH_MOUSE_LL for drag-to-move tiled windows, floating drags and resize.
   2.6 Monitors, DPI and the work area
   • Enumerate with EnumDisplayMonitors; react to WM_DISPLAYCHANGE and WM_DPICHANGED.
   • Use GetMonitorInfo for the work area (monitor minus taskbar).
   • The process must be Per-Monitor DPI Aware v2 (declared in the manifest). Otherwise coordinates are virtualized and windows land in wrong places on mixed-DPI setups.
   2.7 Privilege boundary
   User Interface Privilege Isolation (UIPI) blocks a normal process from manipulating windows of an elevated process. Either run the WM elevated (with its own security trade-offs) or detect elevated windows and leave them floating. v1 will detect and float them, with an opt-in elevated mode later.
   2.8 Workspaces: the hardest platform problem
   Windows has virtual desktops, but the API for them is an undocumented COM interface that changes between Windows builds. Options:
   Approach
   Pros
   Cons
   Native virtual desktops via undocumented COM
   Taskbar integration, native behavior
   Breaks across builds, hard to maintain
   ShowWindow(SW_HIDE)
   Documented
   Windows vanish from taskbar and apps may misbehave on restore
   Minimize
   Documented
   Animations, taskbar flashing, state side effects
   DWM cloaking (DWMWA_CLOAK, undocumented as a setter)
   Clean: window is invisible but still alive
   Undocumented, less stable across builds
   Move off-screen
   Simple
   Hacky, apps can react to being off-screen
   Decision: put workspace visibility behind a small Visibility trait with swappable backends. Prototype cloaking first, keep a fallback backend, and confirm behavior early on the Windows versions we target.
   2.9 What is and is not possible visually
   • Possible: active/inactive border color and rounded corners on Windows 11 via DwmSetWindowAttribute; custom overlay windows (borders, bar) drawn with Direct2D/DirectComposition; animations by overlaying a DWM thumbnail (DwmRegisterThumbnail) of the real window and animating the overlay, then snapping the real window to its final position.
   • Not possible: blur or shadows on third-party windows, true compositor-level animation of real windows.
3. Why existing Windows tilers feel buggy, edgy and slow
   These are the recurring root causes, not mere implementation slips.
4. Trusting events as truth. The internal tree is mutated imperatively in response to each event. Because events are lossy, duplicated and reordered, state slowly drifts from reality. Symptoms: ghost tiles for closed windows, a window missing from layout, wrong focus target, layouts that only recover after a restart.
5. Weak manageability filtering. Deciding "should this window be tiled?" is done with ad-hoc rules, so dialogs, splash screens, UWP helper windows, tool windows and popups get tiled. Users then patch it with ever-growing ignore lists, the classic "edge case" experience.
6. Blocking work in the wrong place. Win32 calls against a hung app, or heavy work in hook callbacks, stall the whole WM or cause Windows to drop the hooks.
7. Per-window, unbatched moves. Applying layout one SetWindowPos at a time causes visible flicker, cascading repaints and slow layout changes.
8. Repeated expensive queries. Re-querying class, process and frame bounds for every window on every event is slow.
9. Entangled design. Layout logic is tangled with Win32 calls, so it cannot be unit tested, and bugs are only found by users on real desktops.
10. Feature sprawl. Animations, bars, many layouts and IPC extras land before the core is solid, multiplying interaction bugs.
11. No reproducibility. A bug report like "it broke when I opened Teams" cannot be replayed, so the same bug returns.
12. How we avoid each problem
    Problem
    Our countermeasure
    Events treated as truth
    Reconciliation: events only mark state dirty. A reconcile loop compares desired state against observed Win32 state and issues minimal corrections. Missed or duplicated events self-heal on the next pass.
    Weak filtering
    A single, pure, tested classify(window_info) -> Manage / Float / Ignore function with an explicit rule order. When uncertain: float, never tile.
    Blocking and slow hooks
    Hook callbacks do one thing: push a tiny event onto a lock-free queue and return. All Win32 queries happen on our worker loop. Moves use async/deferred positioning.
    Unbatched moves
    Each layout change produces one command batch, applied with DeferWindowPos for atomic, flicker-free updates.
    Repeated queries
    A window metadata cache (class, process, styles, frame offsets), invalidated by specific events.
    Entangled design
    Pure core, thin shell. The core is (State, Event) -> (State, Vec<Command>) with no Win32 types. It is fully unit-testable.
    Feature sprawl
    Strict v1 scope. Animations and borders only after the core passes the stability gate.
    No reproducibility
    Record and replay of raw events and snapshots, plus fuzzing of the core.
13. Our approach and architecture
    5.1 Language and stack
    Rust, using the windows crate for Win32 bindings. Reasons: no garbage-collector pauses in latency-sensitive callbacks, strong enums for modeling states that must not be illegal, and good tooling for property-based testing. Suggested crates: windows, crossbeam-channel (queues), serde plus toml (config), tracing (logging and timing), proptest (fuzzing).
    5.2 Process and thread model
    One daemon process with a small number of threads, each with a single job:
    • Hook thread: owns the Win32 message loop, installs SetWinEventHook and the keyboard/mouse hooks. Callbacks only translate and enqueue events.
    • Core thread: the only owner of WM state. Consumes events, runs the pure core, applies commands through the Win32 layer, and runs reconciliation. No locks needed because nothing else touches state.
    • IPC thread: named-pipe server speaking JSON. Requests become events on the core queue, replies come back through a channel.
    • Renderer thread (later): borders and animation overlays via Direct2D/DirectComposition. Receives read-only snapshots, never mutates state.
    • Config watcher: file watcher that sends a reload event.
    5.3 Layered design
14. core crate (pure): monitors, workspaces, layout trees (dwindle, master), window rules, focus logic, commands. Zero Win32 dependency.
15. platform crate (Win32 shell): enumeration, hooks, metadata queries, DeferWindowPos, focus handling, visibility backends, monitor/DPI handling. Implements a trait the core's driver uses, so a fake platform can stand in for tests.
16. daemon crate: wires threads, queues, config, IPC and the reconcile loop.
17. ctl crate: the command-line client (our hyprctl).
    5.4 The core model
    • Monitor { id, work_area, dpi, workspaces }
    • Workspace { id, layout_tree, floating, focused }
    • Layout tree: binary tree for dwindle (split direction and ratio per node), list-plus-master for master layout. Leaves reference window ids.
    • ManagedWindow { id, state: Tiled | Floating | Fullscreen | Pending, rules_applied }
    • Command enum: Move(hwnd, rect), Focus(hwnd), Show(hwnd), Hide(hwnd), SetBorder(...) and so on.
    Use enums so impossible states cannot be represented, for example a window cannot be both tiled and floating.
    5.5 The reconcile loop
    This is the heart of reliability.
18. Events arrive and mark parts of state dirty (for example "workspace 2 on monitor 1 needs re-layout", or "window X needs re-check").
19. On each tick (triggered by events, plus a slow safety interval such as 1 to 2 seconds):
    ◦ Take a snapshot of observed reality: the enumerated manageable windows and their cloaked/visible state, geometry and focus.
    ◦ Diff against desired state: windows we track that no longer exist are removed; new manageable windows are inserted; windows whose geometry deviates from their assigned rect are re-issued.
    ◦ Produce the minimal command batch and apply it in one deferred pass.
20. A window that repeatedly refuses its rectangle is marked stubborn: after N attempts we stop fighting, treat it as floating, and log it, instead of looping forever.
    Because the loop is driven by diffs rather than event order, a missed DESTROY or a duplicated SHOW stops mattering.
    5.6 Manageability policy
    A pure function, evaluated in a fixed order, with a decision and a reason logged for every window:
21. Ignore: invisible, cloaked, zero-size, our own windows, known shell windows.
22. Ignore or float: WS_EX_TOOLWINDOW without WS_EX_APPWINDOW, WS_EX_NOACTIVATE, owned windows with a dialog style.
23. Handle UWP: windows hosted by ApplicationFrameHost are only real when not cloaked and when they have a title and a proper frame.
24. Elevated windows (UIPI): float and leave alone.
25. User rules from config (by class, process, title) override the defaults.
26. Default for unknown cases: Float.
    The decision and reason are exposed through IPC (ctl why <window>), so "why was this tiled?" is always answerable.
    5.7 Geometry correctness
    • Per window, measure GetWindowRect versus DWMWA_EXTENDED_FRAME_BOUNDS once (cached) and inflate the target rect by the invisible borders, so visible edges align with the layout and gaps are exact.
    • Per-Monitor DPI v2 everywhere, with all stored geometry in physical pixels.
    • Handle windows with minimum or maximum sizes by letting the layout engine redistribute the remaining space where possible.
    5.8 Input design
    • Keyboard: start with RegisterHotKey for simple binds. Add a WH_KEYBOARD_LL hook only when submaps or reserved combos are needed, keeping the callback to a table lookup plus an enqueue.
    • Watchdog: periodically check the hook is still alive and re-install if Windows removed it.
    • Mouse: drag handling driven by MOVESIZESTART/END events and a lightweight mouse hook.
    5.9 IPC and config
    • Named pipe with newline-delimited JSON: focus, move, workspace, layout, state, why, reload, dump.
    • Config in TOML (or a Hyprland-like format if preferred), validated on load. A bad config never replaces a good one: on error keep the old config and report the error.
    • Hot reload through the file watcher.
    5.10 Visual layer (after the stability gate)
    • Borders first, using DWMWA_BORDER_COLOR on Windows 11, with overlay windows as a fallback.
    • Animations last: DWM-thumbnail overlay approach, off by default, with an automatic fallback to instant moves if frame time exceeds a budget.
27. Performance budget
    Metric
    Target
    Hook callback duration
    Under 100 microseconds (enqueue only)
    Event to applied layout
    Under 5 ms typical, under 16 ms worst case
    Idle CPU
    Effectively 0% (event-driven, slow safety tick only)
    Memory
    Tens of MB at most
    Startup to first usable layout
    Under 200 ms
    Enforcement: tracing spans on the hook path, the reconcile tick and command application, plus a benchmark harness that replays recorded event streams and reports percentiles. Regressions fail the build.
28. Testing and reliability strategy
29. Unit tests for layout math, classification and focus movement in the pure core.
30. Property-based tests / fuzzing: random event sequences (create, destroy, show, hide, move, monitor changes) against invariants such as:
    ◦ every managed window appears exactly once across all trees,
    ◦ no tiled windows overlap,
    ◦ tiles cover the work area minus gaps,
    ◦ state after reconcile equals state from rebuilding from scratch.
31. Record and replay: a flag that logs every raw event and periodic snapshots to a file. Any bug becomes a replayable test fixture.
32. Fake platform: simulated windows that can refuse sizes, hang, or emit duplicated or reordered events, to test the hard cases without a real desktop.
33. Soak tests: an automated script opening and closing many real apps (Explorer, Notepad, browsers, Electron apps, UWP apps) for hours, then asserting desired state equals observed state.
34. Debug tooling: ctl dump printing desired versus observed state and a list of mismatches, so drift is visible at once.
35. Compatibility matrix: test on Windows 10 and Windows 11, single and multi-monitor, mixed DPI.
36. Build plan
    Phase 0: Spikes and risk reduction (about 1 week)
    Goal: answer the unknowns before committing to the design.
    • Event hook spike: log all window events while using the desktop, to see ordering and noise.
    • Enumerate and classify windows, and print the decision for each.
    • Frame-bounds spike: measure invisible border offsets and verify exact tiling of two windows.
    • Visibility spike: compare cloak, hide and off-screen backends for workspaces.
    • Focus spike: find a reliable way to take foreground.
    • Thumbnail animation spike (time-boxed): confirm feasibility, not polish.
    Exit criteria: written findings for each spike and a decision on the workspace visibility backend.
    Phase 1: Pure core (about 1 to 2 weeks)
    • Core types, dwindle layout, gaps, focus movement, floating, fullscreen.
    • Commands output and the fake platform.
    • Unit tests and the first fuzz tests with invariants.
    Exit criteria: invariants hold under millions of fuzzed events.
    Phase 2: Win32 shell and reconcile loop (about 2 weeks)
    • Hook thread, event queue, metadata cache, classify function.
    • Snapshot, diff and batched DeferWindowPos application.
    • Hotkeys for focus, move, swap, toggle float, close, layout toggle.
    • Record and replay.
    Exit criteria: usable as a daily driver on one monitor with dwindle layout and gaps. Soak test passes for 8 hours.
    Phase 3: Multi-monitor, DPI and workspaces (about 2 weeks)
    • Monitor add/remove, DPI changes, per-monitor workspaces.
    • Workspace switching and moving windows between workspaces with the chosen visibility backend.
    • Reliable handling of sleep/wake, display changes and explorer restarts.
    Exit criteria: hot-plug a monitor and change DPI without losing or misplacing windows.
    Phase 4: Rules, config, IPC (about 1 to 2 weeks)
    • TOML config, validation, hot reload, window rules.
    • Named-pipe IPC and the ctl client, including dump and why.
    • Master layout and submaps.
    Exit criteria: everyday configuration is possible without recompiling. Bad configs never break a running session.
    Stability gate (about 1 week, do not skip)
    • A full week of daily use by you and a few testers, with record mode always on.
    • Fix every drift and edge case found, each with a replay test.
    • Performance benchmarks checked against the budget in section 6.
    No visual features until this gate passes.
    Phase 5: Visuals (about 2 to 3 weeks)
    • Active and inactive borders, rounded corners on Windows 11.
    • Optional animations using the thumbnail overlay approach, behind a flag, with a frame-budget fallback.
    • Optional minimal status bar or IPC hooks for third-party bars.
    Phase 6: Release hardening (ongoing)
    • Installer or portable build, autostart, crash handling (restart safely and re-adopt existing windows).
    • Documentation, default config, issue template asking for a replay log.
    Total rough estimate: about 3 to 4 months part-time to a trusted daily-driver release, shorter for a first usable build after Phase 2.
37. Key risks and mitigations
    Risk
    Mitigation
    Workspace visibility backend breaks on a Windows update
    Backend trait, fallback implementation, version checks, early spike
    Focus switching unreliable
    Dedicated spike, tested technique with fallback, log failures
    Windows removes the low-level keyboard hook
    Tiny callbacks, watchdog and re-install, prefer RegisterHotKey where possible
    Some apps refuse sizes or fight the layout
    Stubborn-window detection, float fallback, per-app rules
    Elevated windows cannot be controlled
    Detect and float, optional elevated mode later
    Scope creep
    Hard v1 scope and the stability gate before visuals
    Animations hurt feel or stability
    Off by default, frame budget, instant-move fallback
    Reliance on undocumented APIs
    Isolate behind traits, document each dependency, test per Windows build
38. Decisions to make before starting
39. Config format: TOML (easy, robust) versus a Hyprland-style syntax (familiar, more parser work).
40. Minimum Windows version: Windows 11 only (simpler borders and corners) versus also Windows 10.
41. Workspace backend: confirmed by the Phase 0 spike.
42. Elevated mode: ship in v1 or defer.
43. Name and repository layout: a Cargo workspace with core, platform, daemon and ctl crates.
44. Immediate next steps
45. Set up the Cargo workspace and the four crates.
46. Complete the Phase 0 spikes, starting with the event hook logger and the classification printout.
47. Write the core types and the first dwindle layout tests, so the pure core exists before any real window is moved.
