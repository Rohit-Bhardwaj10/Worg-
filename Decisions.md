# Decision Register

This file is the durable record of decisions that shape the product or implementation. Add a dated entry when a decision is made or changed. Do not silently convert a proposal into an accepted decision. Use `Proposed`, `Accepted`, `Superseded`, or `Rejected` status. Record the user approval reference when approval is required.

## Decision index

| ID | Topic | Status | Date | Summary |
|---|---|---|---|---|
| D-001 | Product scope | Accepted | 2026-11-17 | Hyprland-inspired Windows tiler; DWM remains compositor; reliability precedes visuals. |
| D-002 | Implementation stack | Accepted | 2026-11-17 | Rust workspace with Win32 isolated in a platform crate. |
| D-003 | Testing workflow | Accepted | 2026-11-17 | Test-first slices; tests are not weakened to hide implementation failures. |
| D-004 | Slice approval gates | Accepted | 2026-11-17 | Report after each slice and wait for explicit approval before the next. |
| D-005 | Commit policy | Accepted | 2026-11-17 | No commits without explicit user permission. |
| D-006 | Initial support baseline | Accepted | 2026-11-17 | Windows 11 x64 development baseline; Windows 10 support unclaimed pending compatibility results. |
| D-007 | Workspace visibility | Open | 2026-11-17 | Select only after backend spike and recovery/compatibility tests. |

## Decision records

### D-001 — Product scope and compositor boundary

- **Status:** Accepted
- **Date:** 2026-11-17
- **Decision:** Build a reliable, Hyprland-inspired tiling window manager for Windows, with dwindle/master layouts, keyboard operation, rules, workspaces, multi-monitor support, config, and IPC as staged capabilities. DWM remains the compositor. Do not promise third-party blur or compositor-level animation.
- **Reason:** Product goal in the user-provided PRD; Windows app controls windows but does not replace DWM.
- **Consequences:** Visual effects remain optional and follow the stability gate; unsupported behavior must have a fallback.

### D-002 — Language and architecture

- **Status:** Accepted
- **Date:** 2026-11-17
- **Decision:** Use stable Rust and a Cargo workspace: pure `wm-core`, Win32 `wm-platform`, orchestration `wm-daemon`, and CLI `wmctl`. Use the `windows` crate at the platform boundary, serde/TOML for config, tracing for diagnostics, and bounded channels. Begin with standard threading/message loops; do not add an async runtime without a demonstrated need.
- **Reason:** The core needs deterministic tests while the shell needs explicit Win32 ownership and failure handling. Slice 0 confirmed the available stable Rust toolchain and Windows MSVC target; CI supplies the complete Windows build environment.
- **Alternatives considered:** A single crate (simpler bootstrap, weaker boundaries); C++ (direct Win32 but less leverage from Rust’s type model); async-first runtime (not required for initial message-loop design).
- **Consequences:** Keep Win32 types out of core; pin toolchain/dependencies and record material dependency changes. Validate crate/API ergonomics in Slice 0.

### D-003 — Test-first development contract

- **Status:** Accepted
- **Date:** 2026-11-17
- **Decision:** For every slice, write detailed unit, integration, property, and applicable Windows end-to-end tests before implementation. Confirm tests expose missing behavior, implement, then rerun focused and full required checks. Fix implementation when tests fail. Do not alter tests merely to obtain green; revise a requirement only through a documented decision and user approval.
- **Reason:** Explicit user instruction.
- **Consequences:** Test design and failing baseline are part of each slice report; unavailable Windows-only checks must be marked unrun, not passed.

### D-004 — One-slice-at-a-time user approval

- **Status:** Accepted
- **Date:** 2026-11-17
- **Decision:** After each completed slice, report delivered behavior, changed files, tests/results, limitations, and proposed next slice; stop and request user approval before beginning that next slice.
- **Reason:** Explicit user instruction.
- **Consequences:** Do not bundle multiple slices into one implementation turn, even when later work appears straightforward.

### D-005 — Commit policy

- **Status:** Accepted
- **Date:** 2026-11-17
- **Decision:** Never create a git commit without explicit user permission.
- **Reason:** Explicit user instruction.
- **Consequences:** A slice can be completed and reviewed as a working-tree diff; ask separately if a commit is desired.

### D-006 — Initial Windows support baseline

- **Status:** Accepted
- **Date:** 2026-11-17
- **Decision:** Use Windows 11 x64 as the initial development and CI baseline. Do not claim Windows 10 support until the relevant API and behavior matrix passes; exact minimum supported Windows build remains open.
- **Reason:** Limits initial compatibility scope while preserving evidence-based expansion.
- **Consequences:** Record OS edition/build and DPI/display setup in Windows test results. This is not yet a release compatibility claim.

### D-007 — Workspace visibility backend

- **Status:** Open
- **Date:** 2026-11-17
- **Decision:** No backend selected yet. Prototype and test documented options behind a visibility interface. `DWMWA_CLOAK` exists in the DWM attribute documentation, but its suitability for general workspace switching must be proven for supported apps/builds, including focus, taskbar, owner/child, app-owned cloak, crash/restart, and recovery behavior.
- **Reason:** Workspace switching is a high-risk product dependency; API existence is not enough to establish correct UX.
- **Consequences:** Slice 8 cannot ship an unverified backend. If no backend meets acceptance criteria, defer or limit workspaces and document the product decision.

## New decision template

### D-XXX — Short decision title

- **Status:** Proposed / Accepted / Superseded / Rejected
- **Date:** YYYY-MM-DD
- **Decision:**
- **Reason:**
- **Alternatives considered:**
- **Consequences:**
- **Evidence/tests:**
- **Approval/reference:**
