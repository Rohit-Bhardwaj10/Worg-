#![cfg(windows)]

use windows_sys::Win32::{
    Foundation::{GetLastError, HWND, RECT},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GWL_EXSTYLE, GWL_STYLE, GetForegroundWindow,
        GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindowVisible, WS_OVERLAPPEDWINDOW,
    },
};
use wm_platform::enumerate_windows;

struct ProbeWindow(HWND);

impl Drop for ProbeWindow {
    fn drop(&mut self) {
        // SAFETY: this test owns the hidden probe window and destroys it during cleanup.
        unsafe { DestroyWindow(self.0) };
    }
}

fn create_probe_window() -> ProbeWindow {
    let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    let title: Vec<u16> = "WinWM read-only probe\0".encode_utf16().collect();
    // SAFETY: the standard STATIC class is registered by USER32; strings are NUL-terminated
    // and remain alive for the duration of the call. The window is created without WS_VISIBLE.
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            71,
            83,
            371,
            283,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        )
    };
    assert!(
        !hwnd.is_null(),
        "probe window creation failed with Win32 error {}",
        // SAFETY: GetLastError only reads this thread's Win32 error state.
        unsafe { GetLastError() }
    );
    ProbeWindow(hwnd)
}

#[test]
fn windows_smoke_enumeration_returns_a_sorted_snapshot_without_commands() {
    let snapshot = enumerate_windows().expect("EnumWindows should return a snapshot");
    assert!(snapshot.captured_at_unix_ms > 0);

    let ids: Vec<_> = snapshot.windows.iter().map(|window| window.id).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        ids, sorted,
        "window handles must be unique and in stable order"
    );

    // The API returns observations only. It has no command list or window-mutating capability.
    for window in &snapshot.windows {
        assert_ne!(window.id.0, 0, "EnumWindows must not report a null HWND");
    }
}

#[test]
fn enumerating_windows_does_not_change_a_controlled_window_state() {
    let probe = create_probe_window();
    let before = read_state(probe.0);
    let snapshot = enumerate_windows().expect("EnumWindows should return a snapshot");
    let after = read_state(probe.0);

    assert_eq!(
        after, before,
        "read-only enumeration must preserve window state"
    );
    let observed = snapshot
        .windows
        .iter()
        .find(|window| window.id.0 == probe.0 as usize as u64)
        .expect("EnumWindows should include the hidden top-level probe");
    assert_eq!(
        observed.visible,
        wm_core::snapshot::Observation::Available(false)
    );
    assert_eq!(
        observed.minimized,
        wm_core::snapshot::Observation::Available(false)
    );
}

fn read_state(hwnd: HWND) -> (i32, i32, i32, i32, bool, bool, isize, isize, HWND) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: the test-owned HWND is valid and rect is writable storage.
    assert_ne!(unsafe { GetWindowRect(hwnd, &mut rect) }, 0);
    // SAFETY: these calls only read state from the test-owned window.
    unsafe {
        (
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            IsWindowVisible(hwnd) != 0,
            IsIconic(hwnd) != 0,
            GetWindowLongPtrW(hwnd, GWL_STYLE),
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE),
            GetForegroundWindow(),
        )
    }
}
