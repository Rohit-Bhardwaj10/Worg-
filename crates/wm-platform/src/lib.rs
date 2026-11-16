//! Read-only Windows observation adapters. Mutating window controls are added in later slices.

pub mod snapshot;
pub use snapshot::{SnapshotError, enumerate_windows};
