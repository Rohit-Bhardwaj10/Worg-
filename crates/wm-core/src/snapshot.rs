use serde::{Deserialize, Serialize};

/// Opaque numeric representation of an HWND; Windows may reuse it after the window is destroyed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowId(pub u64);

/// A query either produced a value or explains why that value could not be read.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum Observation<T> {
    Available(T),
    Unavailable(String),
}

/// Screen coordinates returned by `GetWindowRect`; Windows may virtualize these according to DPI awareness.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Facts gathered about a top-level window. This is observation only; it is not a manageability decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WindowObservation {
    pub id: WindowId,
    pub title: Observation<String>,
    pub class_name: Observation<String>,
    pub process_id: Observation<u32>,
    pub process_path: Observation<String>,
    pub owner: Observation<Option<WindowId>>,
    pub visible: Observation<bool>,
    pub minimized: Observation<bool>,
    pub cloaked: Observation<bool>,
    pub rect: Observation<Rect>,
    pub style: Observation<u64>,
    pub ex_style: Observation<u64>,
}

/// A deterministic diagnostic view of windows observed at a point in time.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub captured_at_unix_ms: u64,
    pub windows: Vec<WindowObservation>,
}

impl WindowSnapshot {
    /// Creates a snapshot with entries ordered by their stable numeric HWND value.
    pub fn new(captured_at_unix_ms: u64, mut windows: Vec<WindowObservation>) -> Self {
        windows.sort_by_key(|window| window.id);
        Self {
            captured_at_unix_ms,
            windows,
        }
    }
}
