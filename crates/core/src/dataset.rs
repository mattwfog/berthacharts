//! Datasets: named, typed columnar data.
//!
//! A [`Dataset`] is a map of `String → Column`. Columns are [`Arc`]-wrapped so
//! they can be cheaply shared across DAG nodes without copies. Transforms
//! produce new Datasets; they never mutate inputs in place.

use std::sync::Arc;

use ahash::AHashMap;

pub use crate::ids::DatasetId;

/// Named, columnar dataset. Immutable — produce a new one from transforms.
#[derive(Debug, Clone)]
pub struct Dataset {
    /// Stable identifier.
    pub id: DatasetId,
    /// Monotonic version bumped whenever contents change.
    pub version: u64,
    /// Columns keyed by name.
    pub columns: AHashMap<String, Arc<Column>>,
    /// Row count (all columns must share this length).
    pub len: usize,
}

impl Dataset {
    /// Build a dataset from a sequence of `(name, column)` pairs.
    ///
    /// All columns must share the same length.
    #[must_use]
    pub fn new(id: DatasetId, version: u64, columns: Vec<(String, Column)>) -> Self {
        let len = columns.first().map(|(_, c)| c.len()).unwrap_or(0);
        let columns: AHashMap<String, Arc<Column>> = columns
            .into_iter()
            .map(|(k, v)| {
                debug_assert_eq!(v.len(), len, "column `{k}` length mismatch");
                (k, Arc::new(v))
            })
            .collect();
        Self {
            id,
            version,
            columns,
            len,
        }
    }

    /// Access a column by name. Returns `None` if the column is absent.
    #[must_use]
    pub fn column(&self, name: &str) -> Option<&Arc<Column>> {
        self.columns.get(name)
    }

    /// Number of rows in the dataset.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// True when the dataset has zero rows.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// A typed column of values.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Column {
    /// 32-bit floating point — the GPU-native numeric type.
    F32(ColumnData<f32>),
    /// 64-bit floating point — kept CPU-side for high-precision domains.
    F64(ColumnData<f64>),
    /// Signed 64-bit integer — used for time (unix millis) and counts.
    I64(ColumnData<i64>),
    /// Unsigned 32-bit integer — categorical codes, packed colors.
    U32(ColumnData<u32>),
    /// Boolean mask.
    Bool(ColumnData<bool>),
    /// Interned string values (category labels).
    Utf8(ColumnData<Arc<str>>),
}

impl Column {
    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Column::F32(d) => d.values.len(),
            Column::F64(d) => d.values.len(),
            Column::I64(d) => d.values.len(),
            Column::U32(d) => d.values.len(),
            Column::Bool(d) => d.values.len(),
            Column::Utf8(d) => d.values.len(),
        }
    }

    /// True when the column has zero entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Static dtype tag for error messages.
    #[must_use]
    pub const fn dtype(&self) -> &'static str {
        match self {
            Column::F32(_) => "f32",
            Column::F64(_) => "f64",
            Column::I64(_) => "i64",
            Column::U32(_) => "u32",
            Column::Bool(_) => "bool",
            Column::Utf8(_) => "utf8",
        }
    }

    /// Read a row as an `f64`, widening from the native dtype.
    ///
    /// `Utf8` columns return `None`. Boolean columns map `true → 1.0` and
    /// `false → 0.0`.
    #[must_use]
    pub fn read_f64(&self, row: usize) -> Option<f64> {
        match self {
            Column::F32(d) => d.values.get(row).map(|v| f64::from(*v)),
            Column::F64(d) => d.values.get(row).copied(),
            Column::I64(d) => d.values.get(row).map(|v| *v as f64),
            Column::U32(d) => d.values.get(row).map(|v| f64::from(*v)),
            Column::Bool(d) => d.values.get(row).map(|v| if *v { 1.0 } else { 0.0 }),
            Column::Utf8(_) => None,
        }
    }
}

/// Storage for a single typed column.
///
/// Values are backed by a `Vec` for CPU access. The renderer uploads a
/// GPU-friendly view at draw time; the column itself is storage-only.
#[derive(Debug, Clone)]
pub struct ColumnData<T> {
    /// Raw values.
    pub values: Vec<T>,
}

impl<T> ColumnData<T> {
    /// Construct from an owned vector.
    #[must_use]
    pub const fn new(values: Vec<T>) -> Self {
        Self { values }
    }
}

/// Registry of datasets owned by a [`crate::Workspace`].
///
/// The registry is append-and-upsert. Datasets are keyed by [`DatasetId`] and
/// carry a monotonic `version` so transforms can cheaply detect changes.
#[derive(Debug, Default, Clone)]
pub struct DatasetRegistry {
    datasets: AHashMap<DatasetId, Arc<Dataset>>,
}

impl DatasetRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a dataset. Returns the previous entry if any.
    pub fn upsert(&mut self, dataset: Dataset) -> Option<Arc<Dataset>> {
        self.datasets.insert(dataset.id, Arc::new(dataset))
    }

    /// Remove a dataset.
    pub fn remove(&mut self, id: DatasetId) -> Option<Arc<Dataset>> {
        self.datasets.remove(&id)
    }

    /// Borrow a dataset by id.
    #[must_use]
    pub fn get(&self, id: DatasetId) -> Option<&Arc<Dataset>> {
        self.datasets.get(&id)
    }

    /// Number of registered datasets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.datasets.len()
    }

    /// True when no datasets are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.datasets.is_empty()
    }

    /// Iterate over all registered datasets.
    pub fn iter(&self) -> impl Iterator<Item = (&DatasetId, &Arc<Dataset>)> {
        self.datasets.iter()
    }
}
