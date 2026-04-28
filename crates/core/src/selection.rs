//! Selection: the shared interaction state that drives coordinated views.
//!
//! A selection represents user interest — highlighted rows, brushed ranges,
//! filter predicates. Transforms can depend on it (e.g. `Filter::bySelection`)
//! so that brushing one panel filters the linked panel. Marks read it during
//! tessellation to render focus / dimming.

use std::ops::Range;

use ahash::AHashMap;
use smallvec::SmallVec;

use crate::ids::{DatasetId, SelectionId};

/// Scalar value used in brush ranges and equality predicates.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Scalar {
    /// Double-precision float.
    F64(f64),
    /// Signed 64-bit integer (also used for time as unix millis).
    I64(i64),
    /// Owned string.
    Utf8(String),
    /// Boolean.
    Bool(bool),
}

/// One channel of the selection — multiple channels ∧ together.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SelectionKind {
    /// Nothing selected.
    Empty,
    /// Explicit list of row indices in a dataset.
    Rows {
        /// Dataset being selected from.
        dataset: DatasetId,
        /// Selected row indices.
        rows: SmallVec<[u32; 16]>,
    },
    /// 1D brush over a numeric/time domain.
    BrushX {
        /// Dataset the brush applies to.
        dataset: DatasetId,
        /// Column the brush applies to.
        column: String,
        /// Half-open range `[lo, hi)` in domain units.
        range: Range<f64>,
    },
    /// 2D rectangular brush over a pair of columns.
    BrushXY {
        /// Dataset the brush applies to.
        dataset: DatasetId,
        /// X-axis column.
        x_column: String,
        /// Y-axis column.
        y_column: String,
        /// X range.
        x_range: Range<f64>,
        /// Y range.
        y_range: Range<f64>,
    },
    /// Arbitrary categorical filter — `value ∈ set`.
    Categorical {
        /// Dataset.
        dataset: DatasetId,
        /// Column.
        column: String,
        /// Allowed values.
        values: Vec<Scalar>,
    },
}

/// A named selection channel.
#[derive(Debug, Clone)]
pub struct SelectionChannel {
    /// Identity.
    pub id: SelectionId,
    /// Current value.
    pub kind: SelectionKind,
    /// Monotonic version — incremented on every change. Feeds DAG invalidation.
    pub version: u64,
}

impl SelectionChannel {
    /// Empty selection with the given id.
    #[must_use]
    pub fn empty(id: SelectionId) -> Self {
        Self {
            id,
            kind: SelectionKind::Empty,
            version: 0,
        }
    }
}

/// Snapshot of all selection channels on a workspace, passed to transforms
/// and marks. Immutable during prepare/render.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    channels: AHashMap<SelectionId, SelectionChannel>,
}

impl Selection {
    /// Empty selection state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow a channel by id.
    #[must_use]
    pub fn get(&self, id: SelectionId) -> Option<&SelectionChannel> {
        self.channels.get(&id)
    }

    /// Iterate all channels.
    pub fn iter(&self) -> impl Iterator<Item = (&SelectionId, &SelectionChannel)> {
        self.channels.iter()
    }

    /// Upsert a channel (internal — wraps workspace-side mutation).
    pub(crate) fn upsert(&mut self, ch: SelectionChannel) -> Option<SelectionChannel> {
        self.channels.insert(ch.id, ch)
    }

    /// Combined version hash across all channels — cheap DAG invalidation key.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut pairs: Vec<_> = self.channels.iter().map(|(k, v)| (*k, v.version)).collect();
        pairs.sort_unstable_by_key(|(k, _)| *k);
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for (k, v) in pairs {
            h ^= u64::from(k.get());
            h = h.wrapping_mul(0x0100_0000_01b3);
            h ^= v;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h
    }
}
