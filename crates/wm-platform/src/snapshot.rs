use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use wm_core::snapshot::WindowSnapshot;

#[derive(Debug)]
pub struct SnapshotError(String);

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SnapshotError {}

/// Read-only snapshot of top-level windows on the current interactive desktop.
#[cfg(windows)]
pub fn enumerate_windows() -> Result<WindowSnapshot, SnapshotError> {
    win32::enumerate_windows()
}

#[cfg(not(windows))]
pub fn enumerate_windows() -> Result<WindowSnapshot, SnapshotError> {
    Err(SnapshotError(
        "window enumeration is only available on Windows".to_owned(),
    ))
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(windows)]
mod win32 {
    use std::{
        ffi::c_void,
        panic::{AssertUnwindSafe, catch_unwind},
    };

    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE, HWND, LPARAM, RECT, SetLastError},
        Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
        System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GetClassNameW, GetWindow,
            GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
            GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
        },
    };
    use wm_core::snapshot::{Observation, Rect, WindowId, WindowObservation, WindowSnapshot};

    use super::{SnapshotError, now_unix_ms};

    struct EnumContext {
        handles: Vec<HWND>,
        callback_panicked: bool,
    }

    /// The lParam is a pointer to the live EnumContext passed below. EnumWindows calls back
    /// synchronously on this thread, so the context remains valid for the entire callback.
    unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        if lparam == 0 {
            return 0;
        }
        // SAFETY: enumerate_windows passes a valid pointer to EnumContext and EnumWindows
        // invokes this callback synchronously before that context goes out of scope.
        let context = unsafe { &mut *(lparam as *mut EnumContext) };
        let result = catch_unwind(AssertUnwindSafe(|| context.handles.push(hwnd)));
        if result.is_err() {
            context.callback_panicked = true;
            0
        } else {
            1
        }
    }

    pub(super) fn enumerate_windows() -> Result<WindowSnapshot, SnapshotError> {
        let mut context = EnumContext {
            handles: Vec::new(),
            callback_panicked: false,
        };
        // SAFETY: the callback and context satisfy EnumWindows' synchronous callback contract.
        unsafe {
            SetLastError(0);
            let succeeded = EnumWindows(
                Some(collect_window),
                (&mut context as *mut EnumContext) as LPARAM,
            );
            if succeeded == 0 && !context.callback_panicked {
                let error = GetLastError();
                if error != 0 {
                    return Err(SnapshotError(format!(
                        "EnumWindows failed with Win32 error {error}"
                    )));
                }
            }
        }
        if context.callback_panicked {
            return Err(SnapshotError(
                "window enumeration callback failed".to_owned(),
            ));
        }

        let windows = context.handles.into_iter().map(observe_window).collect();
        Ok(WindowSnapshot::new(now_unix_ms(), windows))
    }

    fn observe_window(hwnd: HWND) -> WindowObservation {
        let id = WindowId(hwnd as usize as u64);
        let mut process_id = 0u32;
        // SAFETY: hwnd came from EnumWindows; each subsequent API can still fail if it becomes stale.
        let thread_id = unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
        let process_id_observation = if thread_id == 0 {
            Observation::Unavailable(last_error("GetWindowThreadProcessId"))
        } else {
            Observation::Available(process_id)
        };
        let process_path = if thread_id == 0 {
            Observation::Unavailable(
                "window disappeared before its process could be queried".to_owned(),
            )
        } else {
            query_process_path(process_id)
        };

        WindowObservation {
            id,
            title: query_title(hwnd),
            class_name: query_class_name(hwnd),
            process_id: process_id_observation,
            process_path,
            owner: query_owner(hwnd),
            visible: query_bool(hwnd, |handle| {
                // SAFETY: this API only reads visibility for the borrowed handle.
                unsafe { IsWindowVisible(handle) != 0 }
            }),
            minimized: query_bool(hwnd, |handle| {
                // SAFETY: this API only reads minimized state for the borrowed handle.
                unsafe { IsIconic(handle) != 0 }
            }),
            cloaked: query_cloaked(hwnd),
            rect: query_rect(hwnd),
            style: query_style(hwnd, GWL_STYLE, "GWL_STYLE"),
            ex_style: query_style(hwnd, GWL_EXSTYLE, "GWL_EXSTYLE"),
        }
    }

    fn query_bool(hwnd: HWND, query: impl FnOnce(HWND) -> bool) -> Observation<bool> {
        if !is_window(hwnd) {
            Observation::Unavailable("window no longer exists".to_owned())
        } else {
            Observation::Available(query(hwnd))
        }
    }

    fn query_title(hwnd: HWND) -> Observation<String> {
        if !is_window(hwnd) {
            return Observation::Unavailable("window no longer exists".to_owned());
        }
        // SAFETY: this only reads the window caption; an empty caption is a valid value.
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length < 0 {
            return Observation::Unavailable(last_error("GetWindowTextLengthW"));
        }
        let capacity = (length as usize).saturating_add(1);
        if capacity > 1_048_576 {
            return Observation::Unavailable(
                "window title exceeded the 1 Mi UTF-16 safety limit".to_owned(),
            );
        }
        let mut buffer = vec![0u16; capacity.max(1)];
        // SAFETY: buffer has the requested capacity and is writable for the duration of the call.
        let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied == 0 && length > 0 {
            return Observation::Unavailable(last_error("GetWindowTextW"));
        }
        buffer.truncate(copied.max(0) as usize);
        Observation::Available(String::from_utf16_lossy(&buffer))
    }

    fn query_class_name(hwnd: HWND) -> Observation<String> {
        if !is_window(hwnd) {
            return Observation::Unavailable("window no longer exists".to_owned());
        }
        let mut buffer = vec![0u16; 512];
        // SAFETY: buffer is writable and its length is passed as the capacity.
        let copied = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied == 0 {
            return Observation::Unavailable(last_error("GetClassNameW"));
        }
        buffer.truncate(copied as usize);
        Observation::Available(String::from_utf16_lossy(&buffer))
    }

    fn query_owner(hwnd: HWND) -> Observation<Option<WindowId>> {
        if !is_window(hwnd) {
            return Observation::Unavailable("window no longer exists".to_owned());
        }
        // SAFETY: GetWindow with GW_OWNER reads the owner relationship only.
        let owner = unsafe { GetWindow(hwnd, GW_OWNER) };
        Observation::Available((!owner.is_null()).then_some(WindowId(owner as usize as u64)))
    }

    fn query_cloaked(hwnd: HWND) -> Observation<bool> {
        if !is_window(hwnd) {
            return Observation::Unavailable("window no longer exists".to_owned());
        }
        let mut cloaked = 0u32;
        // SAFETY: DwmGetWindowAttribute writes a u32 to the valid local output buffer.
        let result = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                (&mut cloaked as *mut u32).cast::<c_void>(),
                std::mem::size_of::<u32>() as u32,
            )
        };
        if result < 0 {
            Observation::Unavailable(format!(
                "DwmGetWindowAttribute(DWMWA_CLOAKED) failed with HRESULT 0x{:08X}",
                result as u32
            ))
        } else {
            Observation::Available(cloaked != 0)
        }
    }

    fn query_rect(hwnd: HWND) -> Observation<Rect> {
        if !is_window(hwnd) {
            return Observation::Unavailable("window no longer exists".to_owned());
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: GetWindowRect writes to the valid local RECT buffer.
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            Observation::Unavailable(last_error("GetWindowRect"))
        } else {
            Observation::Available(Rect {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            })
        }
    }

    fn query_style(hwnd: HWND, index: i32, name: &str) -> Observation<u64> {
        if !is_window(hwnd) {
            return Observation::Unavailable("window no longer exists".to_owned());
        }
        // SAFETY: GetWindowLongPtrW reads a window style. Clear last error to distinguish a valid zero.
        unsafe { SetLastError(0) };
        let value = unsafe { GetWindowLongPtrW(hwnd, index) };
        if value == 0 {
            let error = unsafe { GetLastError() };
            if error != 0 {
                return Observation::Unavailable(format!(
                    "{name} query failed with Win32 error {error}"
                ));
            }
        }
        Observation::Available(value as u64)
    }

    fn is_window(hwnd: HWND) -> bool {
        // SAFETY: IsWindow only reads the validity of the opaque handle; it does not retain it.
        unsafe { IsWindow(hwnd) != 0 }
    }

    fn query_process_path(process_id: u32) -> Observation<String> {
        // SAFETY: OpenProcess only obtains query rights for the observed process.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if handle.is_null() {
            return Observation::Unavailable(last_error(
                "OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)",
            ));
        }
        let process = ProcessHandle(handle);
        let mut buffer = vec![0u16; 32_768];
        let mut size = buffer.len() as u32;
        // SAFETY: process is a live handle and buffer/size describe writable output storage.
        if unsafe { QueryFullProcessImageNameW(process.0, 0, buffer.as_mut_ptr(), &mut size) } == 0
        {
            return Observation::Unavailable(last_error("QueryFullProcessImageNameW"));
        }
        buffer.truncate(size as usize);
        Observation::Available(String::from_utf16_lossy(&buffer))
    }

    struct ProcessHandle(HANDLE);

    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            // SAFETY: this wrapper owns the valid handle returned by OpenProcess.
            unsafe { CloseHandle(self.0) };
        }
    }

    fn last_error(operation: &str) -> String {
        // SAFETY: GetLastError reads the calling thread's Win32 error state.
        let error = unsafe { GetLastError() };
        format!("{operation} failed with Win32 error {error}")
    }
}

#[cfg(test)]
mod tests {
    use super::now_unix_ms;

    #[test]
    fn capture_timestamp_is_nonzero_and_unix_milliseconds() {
        assert!(now_unix_ms() > 0);
    }
}
