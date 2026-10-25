//! Snapshot cache keyed by scope and data version.

use crate::{AuditScope, Snapshot};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Monotonic counter bumped by the store after every committed write.
#[derive(Debug, Default)]
pub struct DataVersion(AtomicU64);

impl DataVersion {
    /// Current version.
    pub fn current(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }

    /// Marks all cached snapshots stale.
    pub fn bump(&self) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

type Slot = ((AuditScope, u64), Arc<Snapshot>);

/// Small LRU of loaded snapshots.
pub struct SnapshotCache {
    capacity: usize,
    slots: Mutex<VecDeque<Slot>>,
}

impl SnapshotCache {
    /// Cache holding at most `capacity` snapshots.
    pub fn new(capacity: usize) -> Self {
        Self { capacity, slots: Mutex::new(VecDeque::with_capacity(capacity)) }
    }

    /// Cached snapshot for `(scope, version)`, if present.
    pub fn get(&self, scope: &AuditScope, version: u64) -> Option<Arc<Snapshot>> {
        let mut slots = self.slots.lock().ok()?;
        let pos = slots.iter().position(|(k, _)| k.0 == *scope && k.1 == version)?;
        let slot = slots.remove(pos)?;
        let snap = Arc::clone(&slot.1);
        slots.push_front(slot);
        Some(snap)
    }

    /// Stores a snapshot, evicting the least recently used one.
    pub fn put(&self, scope: AuditScope, version: u64, snap: Arc<Snapshot>) {
        let Ok(mut slots) = self.slots.lock() else { return };
        slots.retain(|(k, _)| k.1 == version);
        slots.push_front(((scope, version), snap));
        slots.truncate(self.capacity);
    }
}
