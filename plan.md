# Windows Tiling Window Manager — Build Plan

## Product goal

Build a dependable, keyboard-driven, Hyprland-inspired tiling window manager for Windows. The target is a polished daily driver with dynamic dwindle and master layouts, predictable window rules, workspaces, multi-monitor/DPI support, scriptable control, and safe recovery. Windows remains in charge of composition and window policy; this app observes and controls ordinary top-level windows through supported Win32 APIs where possible.

“Hyprland-level” is the product direction, not a promise that Windows exposes Linux compositor capabilities. DWM remains the compositor; third-party window blur and compositor-level animation are out of scope. Any undocumented or build-sensitive behavior is isolated behind a backend and must have a tested fallback.

## Product principles

1. **Safe by default:** uncertain, elevated, unsupported, or unresponsive windows are left alone or floated; never repeatedly force them into a tile.
2. **Desired state is explicit:** events are hints; reconciliation compares a model with observed desktop state and repairs drift.
3. **Tests define behavior:** write the slice’s acceptance tests before production implementation. A failing test is fixed in the implementation; tests are changed only when the written product requirement is deliberately revised and documented.
4. **One approved slice at a time:** finish, verify, document, and report a slice, then stop for the user’s approval before beginning the next slice.
5. **Recovery is part of correctness:** shutdown, restart, sleep/wake, monitor changes, and partial failures have specified behavior.
6. **No commits without explicit user permission.**

## Proposed stack

- **Language:** stable Rust, edition selected at workspace bootstrap and pinned by `rust-toolchain.toml` once CI is established.
- **Win32 bindings:** `windows` crate, with Win32 calls confined to `wm-platform`. Prefer documented APIs; isolate any undocumented API behind a trait and feature flag.
- **Workspace:** Cargo workspace with `wm-core` (pure state/layout/policy), `wm-platform` (Win32 adapter), `wm-daemon` (runtime and orchestration), and `wmctl` (CLI). Keep crate boundaries small and compile-time enforced.
- **Core data and config:** `serde`, `toml`; validate into a typed immutable config before replacing the active config.
- **Logging/diagnostics:** `tracing` with configurable file output and bounded/rotating logs. Never do formatting or disk I/O in hook callbacks.
- **Concurrency:** core state owned by one core/runtime thread; bounded channels for cross-thread messages. Start with standard channels unless profiling demonstrates a need for a specialized queue. Avoid an async runtime in the initial daemon: Win32 message loops and named-pipe work can be driven by explicit threads. Revisit only with measured need.
- **Testing:** Rust unit/integration tests; property-based tests with `proptest` for core invariants; deterministic fake platform and recorded event fixtures. Add Windows-only integration tests for real API behavior. UI automation/soak harness is a later layer, not a substitute for deterministic tests.
- **Build and CI:** Cargo formatting, clippy, test, and Windows-target build checks. CI must run core tests on every change and Windows integration tests on a Windows runner.

This is the recommended starting stack, not a reason to introduce every dependency at bootstrap. Dependencies and versions are recorded in `Cargo.lock`; additions are justified in `Decisions.md`.

## Proposed architecture

```text
wmctl ── authenticated/session-local named pipe ──> wm-daemon
                                                   │
WinEvent + hotkey message loop ── bounded events ──┤
                                                   ▼
                                            wm-core (pure)
                                                   │ commands
                                                   ▼
                                          wm-platform (Win32)
                                                   │ observations
                                                   └──── reconcile ────┘
```

### `wm-core` — pure and deterministic

- No `HWND`, Win32 imports, filesystem, or global mutable state.
- Typed IDs, physical-pixel rectangles, monitors, workspaces, window lifecycle and management states.
- Pure manageability policy with an auditable decision reason.
- Dwindle tree and master layout algorithms, gaps, minimum-size constraints, focus navigation, floating/fullscreen behavior, workspace membership, rules, and command generation.
- Reducer API conceptually `State + Input -> State + Commands`; reconciliation inputs are explicit observations.
- Invariants: each managed window appears exactly once; no invalid tree references; tiled rectangles stay inside the usable work area; no unintended overlaps; transitions are deterministic.

### `wm-platform` — Win32 boundary

- Enumerate top-level windows and collect metadata; distinguish observed facts from policy decisions.
- WinEvent hook lifecycle and message loop, keyboard registration/hook adapter, display/DPI notifications, foreground/focus adapter, geometry conversion, batched positioning, process privilege detection, and visibility backend implementations.
- Every platform operation returns a result and preserves diagnostics; no silent assumption that a command succeeded.
- `DeferWindowPos` is a batching mechanism, not an atomic/flicker-free transaction guarantee. Handle allocation/defer/end failures and verify results in subsequent observations.
- Per-monitor-v2 awareness is declared in the executable manifest; use physical pixel coordinates consistently and test mixed-DPI transitions.
- Never assume `SWP_ASYNCWINDOWPOS` makes every operation nonblocking or successful. Keep risky platform work off callback paths and measure it.

### `wm-daemon` — lifecycle and orchestration

- Startup snapshot before managing windows; explicit ownership and cancellation for threads/hooks/pipe.
- Hook callbacks enqueue compact events only. The thread installing out-of-context WinEvent hooks owns a message loop; callbacks are designed to tolerate reentrancy and never query window metadata or mutate core state.
- One owner for core state; coalesce event bursts and run event-driven reconciliation plus a slow safety snapshot interval.
- Tag/track manager-issued geometry changes to avoid feedback loops; user drags and application-initiated changes have explicit policies.
- On graceful shutdown restore safe prior state where known; on crash/restart adopt desktop state conservatively and do not blindly replay stale moves.
- IPC uses a per-user/session named pipe with an explicit restrictive security descriptor, bounded message sizes, request validation, and versioned JSON protocol.

### `wmctl` — user and automation surface

- Human-readable commands and structured JSON output; stable error codes.
- Initial scope: `state`, `dump`, `why`, `focus`, `move`, `swap`, `workspace`, `layout`, `reload`, and `version` as each feature becomes available.
- IPC exposes explanations and observed-vs-desired differences needed to debug behavior.

### State reconciliation

Events mark a window, monitor, or workspace dirty. A reconciliation pass gathers a bounded/targeted observation, applies lifecycle and classification policy, computes desired geometry, issues the smallest viable command batch, and records outcomes. A slow full snapshot repairs missed notifications. Repeated failures transition a window to a documented fallback (usually floating/unmanaged) with a reason and retry backoff. No tight retry loop.

### Platform support baseline

Default target is Windows 11 x64, with Windows 10 support treated as a release decision after the first compatibility spike. Do not claim support for a build until the compatibility checklist has passed. Elevated windows are detected and left unmanaged in the first release. Virtual desktop COM integration is not assumed. Workspace visibility backend must pass the experiment and recovery criteria before it is selected.

## Test-first development contract

Every slice follows this loop, in order:

1. Read this plan, `Decisions.md`, `changelog.md`, and the relevant code. Confirm the slice’s scope and dependencies.
2. Write descriptive tests first from the slice’s acceptance criteria. Include unit tests for pure logic, integration tests across crate boundaries, and Windows/platform tests when the behavior depends on Win32. Add failure, boundary, and recovery cases, not only happy paths.
3. Run the new tests and confirm they fail for the intended missing behavior (or document any test that cannot run on the current host).
4. Implement the minimum coherent behavior to satisfy the tests. Do not weaken, delete, skip, or rewrite a failing assertion merely to get green. If a requirement is genuinely wrong, stop and record a proposed change in `Decisions.md`; obtain user approval before changing the acceptance contract.
5. Run focused tests, the full unit/integration suite, formatting, lint, and relevant Windows integration checks. Record exact commands and outcomes. Never report an unrun check as passed.
6. Inspect the diff and verify scope, error handling, logs, and docs. Update decision records and `changelog.md` with the work and test evidence.
7. Report the completed slice, files/behavior changed, tests and results, known limitations, and next slice. Stop. Ask the user to approve beginning the next slice. Do not start it until approval arrives.

Test layers:

- **Unit:** layout geometry, tree operations, classification, rules, config validation, state transitions, focus order, protocol validation.
- **Property/fuzz:** random valid and invalid event sequences; invariants, determinism, and equivalence between incremental reconciliation and rebuilding from a snapshot.
- **Integration:** daemon/core/platform adapter seams using a fake platform; IPC request-response and error paths; startup/shutdown/restart adoption.
- **Windows integration:** real window enumeration, geometry/frame bounds, foreground restrictions, DPI and display change handling, hook delivery/lifecycle, and chosen visibility behavior. Tests must use controlled test windows and clean up safely.
- **End-to-end/soak:** scripted open/close/focus/drag/workspace/monitor scenarios, long-run churn, app refusal/failure, explorer restart, sleep/wake, and recovery. Preserve logs and replayable event fixtures.

## Vertical slices and approval gates

Each slice delivers a user-observable or operationally complete increment. Slices are sequential; dependencies are not started early. The test list below is a minimum, not an exhaustive ceiling. Expand it before implementation.

### Slice 0 — repository baseline and decision register

**Deliverable:** Rust workspace skeleton, reproducible build commands, CI baseline, root guidance, and decision/change tracking.

**Tests/checks first:** workspace metadata/build check; CI workflow validation; verify clean checkout can build all empty crates on Windows; docs links/structure review.

**Acceptance:** crate boundaries compile; toolchain and commands are documented; CI runs format, lint, tests, and Windows build; no platform behavior is enabled. Record stack and support-baseline decisions.

### Slice 1 — safe observation and diagnostic snapshot

**Deliverable:** read-only daemon enumerates visible top-level windows and emits a diagnostic snapshot; it does not move, hide, focus, or alter windows.

**Tests first:** fake enumeration fixtures for ordinary windows, owned dialogs, tool windows, cloaked/minimized/invisible windows, stale HWNDs, inaccessible/elevated processes, duplicate notifications; snapshot determinism and serialization; Windows smoke test confirms no geometry/state changes.

**Acceptance:** `wmctl dump` explains each observed candidate and its facts; startup/shutdown leaves desktop unchanged; failures are explicit and logged.

### Slice 2 — pure policy and classification

**Deliverable:** deterministic `Manage | Float | Ignore` decisions with reason codes and ordered user overrides (initially test/config fixture driven).

**Tests first:** table-driven rule precedence, owner/dialog/style cases, UWP host cases, elevated/unknown fallback, own-process/shell exclusions, overlapping rules, case handling, invalid rule behavior, property tests for stable outcomes.

**Acceptance:** no HWND is managed without an explainable policy result; uncertain cases use the documented safe fallback; `why` output is tested.

### Slice 3 — pure layout engine and command plan

**Deliverable:** dwindle layout with gaps, insert/remove/swap/focus operations, and deterministic geometry plan for synthetic monitors/windows. No real windows moved yet.

**Tests first:** one/two/many windows, odd dimensions, gap accounting, insert direction, removal/collapse, focus wrap, minimum-size conflicts, empty/singleton state, monitor work-area edges; property tests for exactly-once membership, bounds, overlaps, and determinism.

**Acceptance:** core produces valid rectangles and typed commands under all defined cases; conflicts return an explicit fallback/result, never invalid geometry.

### Slice 4 — controlled single-monitor real tiling

**Deliverable:** opt-in foreground/managed window admission and single-monitor dwindle; safe quit/restore path; basic configurable hotkeys with documented defaults.

**Tests first:** fake-platform integration for create/show/destroy/minimize, refused move, stale HWND, manager-generated location event, app resize fight, move failure, shutdown restore; Windows test with controlled windows verifies visible frame alignment and cleanup.

**Acceptance:** ordinary windows tile and retile predictably; floats remain untouched; failures stop retries and float/leave unmanaged; user can immediately disable/exit; measured layout and move errors are diagnosable.

### Slice 5 — reliable event loop and reconciliation

**Deliverable:** WinEvent observation, bounded event queue, coalescing, dirty state, periodic safety snapshot, event recording and replay.

**Tests first:** event duplication/reordering/bursts, missed destroy recovered by snapshot, callback reentrancy model, queue saturation policy, feedback from manager moves, hung/refusing window simulation, replay equivalence, no tight retry loops.

**Acceptance:** desired state converges to observed reality for supported windows; callbacks only enqueue compact data; reconciliation has bounded work and recovery diagnostics. Windows soak scenario produces no ghost entries.

### Slice 6 — focus, floating, fullscreen, and robust input

**Deliverable:** reliable focus navigation, toggle float, fullscreen handling, move/swap actions, key binding validation and fallback behavior.

**Tests first:** focus order under add/remove, foreground denial, minimized/hidden targets, fullscreen/restoration, binding collisions/reserved combinations, hook loss/reinstall behavior, callback timing budget, key repeat/modifier edge cases.

**Acceptance:** foreground restrictions are handled without focus-stealing loops; supported keys are reported at startup; input hook callbacks remain minimal; RegisterHotKey is preferred when adequate.

### Slice 7 — multi-monitor and DPI correctness

**Deliverable:** per-monitor workspace assignment, monitor add/remove/reorder, work-area changes, per-monitor-v2 manifest, DPI-aware geometry.

**Tests first:** synthetic monitor topology changes and pure migration policy; physical-pixel round trips; mixed DPI and negative coordinates; unplug of active monitor; taskbar/work-area change; WM_DISPLAYCHANGE/WM_DPICHANGED sequences.

**Acceptance:** windows remain visible and assigned deterministically after topology changes; no stale monitor geometry; controlled Windows tests pass on single/mixed-DPI configurations.

### Slice 8 — workspace visibility backend

**Deliverable:** workspaces with an experimentally validated visibility backend, isolated behind a trait; explicit fallback/unsupported state.

**Tests first:** backend contract, show/hide failure, app-owned cloak, taskbar/focus behavior, switching with dialogs/popups, crash/restart recovery, notification behavior, backend unavailable/build mismatch.

**Acceptance:** selected backend has written findings and passes matrix on each claimed Windows version. No backend is selected solely on API availability. If no safe backend meets criteria, ship workspace switching as deferred/limited and record decision.

### Slice 9 — configuration and user rules

**Deliverable:** versioned TOML config, validation, atomic apply/reload, rules by process/class/title, hotkeys/layout/gaps settings.

**Tests first:** parsing, defaults, unknown/invalid values, rule precedence, malformed reload retains last good config, file replacement races, paths/encoding, duplicate bindings, config migration/version handling.

**Acceptance:** bad config cannot damage running state; errors identify file/field; reload effects are deterministic and logged.

### Slice 10 — named-pipe IPC and complete `wmctl`

**Deliverable:** documented, versioned local IPC and CLI for state/dump/why, focus/move/swap, workspace/layout, reload, version.

**Tests first:** protocol round trips, malformed/oversized messages, unknown commands/version, concurrent clients, daemon unavailable/restart, access control, pipe cleanup, command authorization/scope, stable JSON and human output.

**Acceptance:** only the current user/session can access the pipe; every mutating command validates inputs and returns explicit success/failure; CLI can diagnose daemon state without hanging.

### Slice 11 — master layout and interaction polish

**Deliverable:** master layout and layout switching, full supported navigation/move workflow, documented defaults suitable for daily use.

**Tests first:** master count/ratio boundaries, insertion/removal, focus and swap semantics, toggling layouts preserves windows and focus, min-size constraints, config reload while active.

**Acceptance:** both layouts satisfy shared geometry invariants and user operations have consistent semantics.

### Slice 12 — hardening, compatibility, and release candidate

**Deliverable:** crash-safe startup/adoption, autostart/portable packaging choice, diagnostics bundle, documented compatibility matrix, long-run validation.

**Tests first:** restart at every lifecycle point, explorer restart, sleep/wake, log rotation/disk full, pipe denial, display change during layout, uptime/churn soak, no leaked hooks/threads/handles, upgrade/config compatibility.

**Acceptance:** agreed stability period passes with zero unresolved state drift or crash-class defects; all supported platform scenarios pass; release/rollback instructions are clear. Release is not automatic; obtain explicit user approval before publishing/distribution.

### Slice 13 — optional visuals after stability gate

**Deliverable:** active/inactive borders and optional visual enhancements only if supported APIs behave safely. Animations are a separate opt-in experiment.

**Tests first:** DWM attribute failure/version fallback, app compatibility, DPI/color changes, resource cleanup, focus changes, performance budgets, animation interruption/disable path.

**Acceptance:** visuals cannot affect core state or block input; instant geometry remains fallback; feature is disabled when unsupported. No blur claim for third-party windows.

## Cross-slice definition of done

- Acceptance tests were written before implementation and mapped to the slice.
- Focused and full required test layers have recorded results; failures are fixed in implementation, not hidden by weakening tests.
- Formatting/lint/build checks required by CI pass, or precise blockers are recorded.
- No unsafe behavior is silently ignored; errors and fallback reasons are observable.
- Relevant docs, `Decisions.md`, and `changelog.md` are updated.
- No commit has been made without explicit user permission.
- User receives a concise completion report and is asked to approve the next slice; work stops at that gate.

## Success measures

Measure separately: callback duration, core computation duration, command submission duration, and time to observed geometry. Establish baselines before setting hard performance gates. Track idle CPU, memory, missed/replayed events, failed moves, convergence time, crashes, and unexplained drift. A numerical target is not considered met until the measurement method and test setup are documented.

## Source references for platform constraints

- [SetWinEventHook](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook): out-of-context callbacks run on the installing thread, which requires a message loop; callbacks can be reentrant.
- [DWMWINDOWATTRIBUTE](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute): documents `DWMWA_CLOAK` as a setter and `DWMWA_CLOAKED` as a getter; general workspace suitability still requires testing.
- [BeginDeferWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-begindeferwindowpos): deferred positioning can fail; batching is not described as a transactional visual guarantee.
- [DPI awareness](https://learn.microsoft.com/en-us/windows/win32/hidpi/setting-the-default-dpi-awareness-for-a-process): Microsoft recommends setting process DPI awareness in the manifest.
- [Named-pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights): pipe access is governed by a security descriptor; choose a restrictive per-user/session DACL.
