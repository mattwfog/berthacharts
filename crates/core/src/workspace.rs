//! Workspace: shared state that multiple [`Chart`]s can subscribe to.
//!
//! A workspace owns:
//!
//! - The [`ScaleRegistry`] — all named scales.
//! - The [`DatasetRegistry`] — all raw source datasets.
//! - Coord-system instances keyed by [`CoordId`].
//! - The [`Selection`] state used for coordinated views.
//! - A registry of event listeners.
//!
//! Workspaces are always held behind an [`Arc`]. Interior mutability via
//! [`RwLock`] keeps the API uniform across single-threaded WASM and native
//! multi-threaded targets.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use ahash::AHashMap;

use crate::coord::{Coord, CoordId};
use crate::dataset::{Dataset, DatasetId, DatasetRegistry};
use crate::event::{ChartEvent, Subscription};
use crate::ids::ScaleId;
use crate::scale::{Scale, ScaleRegistry};
use crate::selection::{Selection, SelectionChannel};

type Listener = Arc<dyn Fn(&ChartEvent) + Send + Sync + 'static>;

struct Inner {
    scales: ScaleRegistry,
    datasets: DatasetRegistry,
    coords: AHashMap<CoordId, Arc<dyn Coord>>,
    selection: Selection,
    listeners: AHashMap<u64, Listener>,
}

/// Shared chart state. Build with [`Workspace::new`].
pub struct Workspace {
    inner: RwLock<Inner>,
    next_listener_id: AtomicU64,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace").finish_non_exhaustive()
    }
}

impl Workspace {
    /// Build a new, empty workspace wrapped in an [`Arc`].
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: RwLock::new(Inner {
                scales: ScaleRegistry::new(),
                datasets: DatasetRegistry::new(),
                coords: AHashMap::new(),
                selection: Selection::new(),
                listeners: AHashMap::new(),
            }),
            next_listener_id: AtomicU64::new(1),
        })
    }

    // ------- Scales -------

    /// Insert or replace a scale.
    pub fn upsert_scale(&self, id: ScaleId, scale: Arc<dyn Scale>) {
        self.inner
            .write()
            .expect("workspace write")
            .scales
            .upsert(id, scale);
        self.emit(&ChartEvent::ScaleChanged(id));
    }

    /// Borrow a scale by id (returns a cloned `Arc`).
    #[must_use]
    pub fn scale(&self, id: ScaleId) -> Option<Arc<dyn Scale>> {
        self.inner
            .read()
            .expect("workspace read")
            .scales
            .get(id)
            .cloned()
    }

    // ------- Datasets -------

    /// Insert or replace a source dataset.
    pub fn upsert_dataset(&self, dataset: Dataset) {
        let id = dataset.id;
        self.inner
            .write()
            .expect("workspace write")
            .datasets
            .upsert(dataset);
        self.emit(&ChartEvent::DatasetChanged(id));
    }

    /// Borrow a dataset by id.
    #[must_use]
    pub fn dataset(&self, id: DatasetId) -> Option<Arc<Dataset>> {
        self.inner
            .read()
            .expect("workspace read")
            .datasets
            .get(id)
            .cloned()
    }

    // ------- Coord systems -------

    /// Insert or replace a coord system.
    pub fn upsert_coord(&self, id: CoordId, coord: Arc<dyn Coord>) {
        self.inner
            .write()
            .expect("workspace write")
            .coords
            .insert(id, coord);
    }

    /// Borrow a coord system.
    #[must_use]
    pub fn coord(&self, id: CoordId) -> Option<Arc<dyn Coord>> {
        self.inner
            .read()
            .expect("workspace read")
            .coords
            .get(&id)
            .cloned()
    }

    // ------- Selection -------

    /// Upsert a selection channel.
    pub fn upsert_selection(&self, ch: SelectionChannel) {
        let id = ch.id;
        self.inner
            .write()
            .expect("workspace write")
            .selection
            .upsert(ch);
        self.emit(&ChartEvent::SelectionChanged(id));
    }

    /// Snapshot the current selection state (cloned — cheap: small Arcs).
    #[must_use]
    pub fn selection(&self) -> Selection {
        self.inner.read().expect("workspace read").selection.clone()
    }

    /// Snapshot the scale registry (cloned — cheap: `Arc` values).
    #[must_use]
    pub fn scales(&self) -> crate::scale::ScaleRegistry {
        self.inner.read().expect("workspace read").scales.clone()
    }

    /// Snapshot the dataset registry (cloned — cheap: `Arc` values).
    #[must_use]
    pub fn datasets(&self) -> crate::dataset::DatasetRegistry {
        self.inner.read().expect("workspace read").datasets.clone()
    }

    // ------- Events -------

    /// Subscribe to chart events. Drop the returned [`Subscription`] to
    /// unsubscribe.
    pub fn subscribe<F>(self: &Arc<Self>, f: F) -> Subscription
    where
        F: Fn(&ChartEvent) + Send + Sync + 'static,
    {
        let id = self.next_listener_id.fetch_add(1, Ordering::Relaxed);
        self.inner
            .write()
            .expect("workspace write")
            .listeners
            .insert(id, Arc::new(f));
        let weak = Arc::downgrade(self);
        Subscription::new(move || {
            if let Some(ws) = weak.upgrade() {
                if let Ok(mut inner) = ws.inner.write() {
                    inner.listeners.remove(&id);
                }
            }
        })
    }

    /// Fire an event to all listeners. Internal — bindings emit via mutating
    /// APIs above.
    pub(crate) fn emit(&self, event: &ChartEvent) {
        let listeners: Vec<Listener> = {
            let inner = match self.inner.read() {
                Ok(g) => g,
                Err(_) => return,
            };
            inner.listeners.values().cloned().collect()
        };
        for l in listeners {
            l(event);
        }
    }
}
