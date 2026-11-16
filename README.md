# WinWM

A Hyprland-inspired tiling window manager for Windows. See [plan.md](plan.md) for scope and vertical slices, [Decisions.md](Decisions.md) for recorded decisions, and [changelog.md](changelog.md) for current handoff context.

## Prerequisites

- Rust stable with the `x86_64-pc-windows-msvc` target, rustfmt, and clippy (the repository toolchain file requests these).
- Windows 11 x64 is the initial development target. Windows 10 support is not yet claimed.

## Build and check

From the repository root:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace --all-targets
.\tests\bootstrap_contract.ps1
```

The current workspace is a compile-only skeleton. Window management behavior is added in the approved vertical slices; no window is controlled by this baseline.
