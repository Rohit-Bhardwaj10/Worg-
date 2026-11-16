# Changelog and Handoff Context

This file carries concise project context between work sessions and records completed changes. Keep newest entries first. Do not claim a feature or test is complete unless it has been implemented or actually run. Detailed product direction lives in `plan.md`; durable choices live in `Decisions.md`.

## Current project state

- **Product:** Hyprland-inspired tiling window manager for Windows; reliability and safe recovery are core requirements.
- **Current phase:** Slice 0 bootstrap complete; no window management behavior exists yet.
- **Next authorized work:** Slice 1 in `plan.md` (safe observation and diagnostic snapshot), pending user approval.
- **Stack:** Rust stable/Cargo workspace with four crates is established; MSVC linker is missing from the local environment, so CI is the full-link/test validation path until build tools are available.
- **Open high-risk decision:** Workspace visibility backend; see D-007 in `Decisions.md`.
- **Workflow:** Tests first per slice; implement; run focused and full checks; update this file and decisions; report slice; stop for approval before next slice.
- **Commit policy:** No commits without explicit user permission.

## Entries

### 2026-11-17 — Slice 0: repository baseline

- **Delivered:** Four-crate Cargo workspace (`wm-core`, `wm-platform`, `wm-daemon`, `wmctl`), stable Rust toolchain file, lockfile, README build instructions, CI workflow for Windows x64 MSVC, bootstrap contract check, and `/target/` ignore rule. No platform behavior is active.
- **Prerequisites checked:** Rust/Cargo 1.97.1, stable `x86_64-pc-windows-msvc`, rustfmt, clippy, Git 2.49.0, PowerShell 7.6.10, GitHub CLI available. MSVC linker/build tools are not installed; `link.exe` resolves to Git's utility, not Microsoft's linker. No `.git` repository exists; none was initialized.
- **Tests written first:** `tests/bootstrap_contract.ps1`; first run failed as expected because `Cargo.toml` was absent. After workspace setup it passed.
- **Checks:** `cargo fmt --all -- --check` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `cargo check --workspace --all-targets` passed; `cargo metadata --no-deps --format-version 1` passed; bootstrap contract passed. `cargo test --workspace --all-targets` and `cargo build --workspace --all-targets` attempted but could not link because Microsoft C++ build tools/linker are absent. CI is configured to execute them on `windows-latest`.
- **Decision records:** D-002 stack/architecture and D-006 initial development baseline accepted. Windows 10 release support remains unclaimed; workspace visibility remains open.
- **Known limitation:** Local test binaries and executable builds remain unverified until MSVC Build Tools are available or CI runs. The workspace contains placeholders only.
- **Next slice proposed:** Slice 1 — safe observation and diagnostic snapshot.
- **Approval status:** Awaiting user approval.

### 2026-11-17 — Planning documents created

- Added `plan.md` with proposed stack/architecture, test-first contract, cross-slice done criteria, and 14 sequential vertical slices from workspace bootstrap through optional visuals.
- Added `Decisions.md` with initial scope/workflow decisions and explicit proposed/open technical choices.
- Added root `AGENTS.md` with project-wide agent instructions, slice gates, test-first rules, documentation requirements, and no-commit rule.
- Reviewed platform assumptions against Microsoft documentation: WinEvent callback thread/message-loop and reentrancy behavior; `DWMWA_CLOAK`/`DWMWA_CLOAKED`; deferred-positioning failure semantics; manifest-based DPI awareness; named-pipe security descriptors.
- **Tests:** No application code or tests exist yet; no test suite was run for this documentation-only planning change.
- **Next:** Slice 0 has been completed; see the entry above.

## Entry template

### YYYY-MM-DD — Slice N: title

- **Delivered:**
- **Files/areas changed:**
- **Tests written first:**
- **Checks run and results:**
- **Known limitations/open decisions:**
- **Decision records updated:**
- **Next slice proposed:**
- **Approval status:** Awaiting user approval / Approved / Not requested
