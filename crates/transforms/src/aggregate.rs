//! Group-by aggregation over a single value column.
//!
//! Partitions rows by the values in `group_by`, then reduces each partition's
//! `value` column into a single number using [`AggOp`]. Output dataset has one
//! row per group, with the group-key column preserved (as `Utf8` for string
//! keys, otherwise as `F64` for numeric keys) plus the aggregated value column.
//!
//! v0.1 supports a single group column and a single aggregation. Multi-key /
//! multi-output aggregation will land alongside join/groupby DAG plumbing.

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Arc;

use berthacharts_core::{
    Column, ColumnData, Dataset, Selection, Transform, TransformError, TransformInputs,
    TransformOutput,
};

use crate::fingerprint::{derive_output_dataset_id, Hasher};

const NAME: &str = "aggregate";

/// Reduction applied to each group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggOp {
    /// Sum of values.
    Sum,
    /// Arithmetic mean.
    Mean,
    /// Minimum value.
    Min,
    /// Maximum value.
    Max,
    /// Number of rows in the group (ignores the value column).
    Count,
}

impl AggOp {
    fn tag(self) -> u64 {
        match self {
            AggOp::Sum => 1,
            AggOp::Mean => 2,
            AggOp::Min => 3,
            AggOp::Max => 4,
            AggOp::Count => 5,
        }
    }
}

/// Group rows by `group_by` and reduce `value` with `op` into `output`.
#[derive(Debug, Clone)]
pub struct Aggregate {
    /// Column whose values define the groups.
    pub group_by: String,
    /// Column to aggregate. Ignored when `op == AggOp::Count`.
    pub value: String,
    /// Reduction to apply.
    pub op: AggOp,
    /// Name of the output aggregated column.
    pub output: String,
}

impl Aggregate {
    /// Construct a new aggregation.
    #[must_use]
    pub fn new(
        group_by: impl Into<String>,
        value: impl Into<String>,
        op: AggOp,
        output: impl Into<String>,
    ) -> Self {
        Self {
            group_by: group_by.into(),
            value: value.into(),
            op,
            output: output.into(),
        }
    }
}

impl Transform for Aggregate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn fingerprint(&self) -> u64 {
        let mut h = Hasher::new();
        h.write_str(NAME)
            .write_str(&self.group_by)
            .write_str(&self.value)
            .write_u64(self.op.tag())
            .write_str(&self.output);
        h.finish()
    }

    fn run(
        &self,
        inputs: TransformInputs<'_>,
        _selection: &Selection,
    ) -> Result<TransformOutput, TransformError> {
        if inputs.len() != 1 {
            return Err(TransformError::Arity {
                expected: 1,
                actual: inputs.len(),
            });
        }
        let input = &inputs[0];
        let group_col = input
            .column(&self.group_by)
            .ok_or_else(|| TransformError::MissingColumn(self.group_by.clone()))?;
        // For Count, the value column is unused, but for any reducer that reads
        // numbers it must be present and numeric.
        let value_col = if self.op == AggOp::Count {
            None
        } else {
            Some(
                input
                    .column(&self.value)
                    .ok_or_else(|| TransformError::MissingColumn(self.value.clone()))?,
            )
        };

        let group_kind = GroupKind::detect(group_col)?;

        // Use BTreeMap so output ordering is stable across runs.
        let mut groups_str: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut groups_num: BTreeMap<u64, (f64, Vec<usize>)> = BTreeMap::new();

        for row in 0..input.len {
            match group_kind {
                GroupKind::Utf8 => {
                    if let Some(s) = read_utf8(group_col, row) {
                        groups_str.entry(s).or_default().push(row);
                    }
                }
                GroupKind::Numeric => {
                    if let Some(v) = group_col.read_f64(row) {
                        if v.is_finite() {
                            groups_num
                                .entry(v.to_bits())
                                .or_insert((v, Vec::new()))
                                .1
                                .push(row);
                        }
                    }
                }
            }
        }

        let mut group_keys_str: Vec<Arc<str>> = Vec::new();
        let mut group_keys_num: Vec<f64> = Vec::new();
        let mut agg_values: Vec<f64> = Vec::new();

        match group_kind {
            GroupKind::Utf8 => {
                for (key, rows) in groups_str {
                    let v = reduce(self.op, value_col, &rows);
                    group_keys_str.push(Arc::from(key.as_str()));
                    agg_values.push(v);
                }
            }
            GroupKind::Numeric => {
                for (_, (key_value, rows)) in groups_num {
                    let v = reduce(self.op, value_col, &rows);
                    group_keys_num.push(key_value);
                    agg_values.push(v);
                }
            }
        }

        let mut columns: Vec<(String, Column)> = Vec::with_capacity(2);
        match group_kind {
            GroupKind::Utf8 => {
                columns.push((
                    self.group_by.clone(),
                    Column::Utf8(ColumnData::new(group_keys_str)),
                ));
            }
            GroupKind::Numeric => {
                columns.push((
                    self.group_by.clone(),
                    Column::F64(ColumnData::new(group_keys_num)),
                ));
            }
        }
        columns.push((
            self.output.clone(),
            Column::F64(ColumnData::new(agg_values)),
        ));

        let next_id = derive_output_dataset_id(self.fingerprint(), input.id);
        let next_version = input.version.wrapping_add(1);
        Ok(Arc::new(Dataset::new(next_id, next_version, columns)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, Copy)]
enum GroupKind {
    Utf8,
    Numeric,
}

impl GroupKind {
    fn detect(col: &Arc<Column>) -> Result<Self, TransformError> {
        match col.as_ref() {
            Column::Utf8(_) => Ok(GroupKind::Utf8),
            Column::F32(_) | Column::F64(_) | Column::I64(_) | Column::U32(_) | Column::Bool(_) => {
                Ok(GroupKind::Numeric)
            }
            // `Column` is `#[non_exhaustive]` — treat unknown future dtypes
            // as numeric so `read_f64` (which gracefully returns `None` for
            // non-numeric variants) drives behaviour at the call site.
            _ => Ok(GroupKind::Numeric),
        }
    }
}

fn read_utf8(col: &Arc<Column>, row: usize) -> Option<String> {
    match col.as_ref() {
        Column::Utf8(d) => d.values.get(row).map(|s| s.to_string()),
        _ => None,
    }
}

fn reduce(op: AggOp, value_col: Option<&Arc<Column>>, rows: &[usize]) -> f64 {
    if rows.is_empty() {
        return match op {
            AggOp::Count => 0.0,
            _ => f64::NAN,
        };
    }
    match op {
        AggOp::Count => rows.len() as f64,
        AggOp::Sum => rows
            .iter()
            .filter_map(|&r| value_col.and_then(|c| c.read_f64(r)))
            .sum(),
        AggOp::Mean => {
            let xs: Vec<f64> = rows
                .iter()
                .filter_map(|&r| value_col.and_then(|c| c.read_f64(r)))
                .collect();
            if xs.is_empty() {
                f64::NAN
            } else {
                xs.iter().sum::<f64>() / xs.len() as f64
            }
        }
        AggOp::Min => rows
            .iter()
            .filter_map(|&r| value_col.and_then(|c| c.read_f64(r)))
            .fold(f64::INFINITY, f64::min),
        AggOp::Max => rows
            .iter()
            .filter_map(|&r| value_col.and_then(|c| c.read_f64(r)))
            .fold(f64::NEG_INFINITY, f64::max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Column, ColumnData, DatasetId};

    fn categorical_fixture() -> Arc<Dataset> {
        Arc::new(Dataset::new(
            DatasetId::new(10),
            0,
            vec![
                (
                    "group".into(),
                    Column::Utf8(ColumnData::new(vec![
                        Arc::from("a"),
                        Arc::from("b"),
                        Arc::from("a"),
                        Arc::from("b"),
                        Arc::from("a"),
                    ])),
                ),
                (
                    "value".into(),
                    Column::F64(ColumnData::new(vec![1.0, 10.0, 2.0, 20.0, 3.0])),
                ),
            ],
        ))
    }

    fn out_f64(ds: &Dataset, name: &str) -> Vec<f64> {
        match ds.column(name).unwrap().as_ref() {
            Column::F64(d) => d.values.clone(),
            _ => panic!("expected f64 column"),
        }
    }

    fn out_utf8(ds: &Dataset, name: &str) -> Vec<String> {
        match ds.column(name).unwrap().as_ref() {
            Column::Utf8(d) => d.values.iter().map(|s| s.to_string()).collect(),
            _ => panic!("expected utf8 column"),
        }
    }

    #[test]
    fn sum_groups_categorical() {
        let agg = Aggregate::new("group", "value", AggOp::Sum, "total");
        let out = agg
            .run(&[categorical_fixture()], &Selection::new())
            .unwrap();
        // BTreeMap key order ⇒ "a" then "b"
        assert_eq!(out_utf8(&out, "group"), vec!["a", "b"]);
        assert_eq!(out_f64(&out, "total"), vec![1.0 + 2.0 + 3.0, 10.0 + 20.0]);
    }

    #[test]
    fn mean_groups_categorical() {
        let agg = Aggregate::new("group", "value", AggOp::Mean, "avg");
        let out = agg
            .run(&[categorical_fixture()], &Selection::new())
            .unwrap();
        assert_eq!(out_f64(&out, "avg"), vec![2.0, 15.0]);
    }

    #[test]
    fn min_max_groups_categorical() {
        let mn = Aggregate::new("group", "value", AggOp::Min, "lo")
            .run(&[categorical_fixture()], &Selection::new())
            .unwrap();
        let mx = Aggregate::new("group", "value", AggOp::Max, "hi")
            .run(&[categorical_fixture()], &Selection::new())
            .unwrap();
        assert_eq!(out_f64(&mn, "lo"), vec![1.0, 10.0]);
        assert_eq!(out_f64(&mx, "hi"), vec![3.0, 20.0]);
    }

    #[test]
    fn count_ignores_missing_value_column() {
        let agg = Aggregate::new("group", "doesnt_matter", AggOp::Count, "n");
        let out = agg
            .run(&[categorical_fixture()], &Selection::new())
            .unwrap();
        assert_eq!(out_f64(&out, "n"), vec![3.0, 2.0]);
    }

    #[test]
    fn missing_value_column_errors_for_non_count() {
        let agg = Aggregate::new("group", "missing", AggOp::Sum, "total");
        let err = agg
            .run(&[categorical_fixture()], &Selection::new())
            .unwrap_err();
        assert!(matches!(err, TransformError::MissingColumn(c) if c == "missing"));
    }

    #[test]
    fn missing_group_column_errors() {
        let agg = Aggregate::new("nope", "value", AggOp::Sum, "total");
        let err = agg
            .run(&[categorical_fixture()], &Selection::new())
            .unwrap_err();
        assert!(matches!(err, TransformError::MissingColumn(c) if c == "nope"));
    }

    #[test]
    fn numeric_group_keys_supported() {
        let ds = Arc::new(Dataset::new(
            DatasetId::new(11),
            0,
            vec![
                (
                    "k".into(),
                    Column::I64(ColumnData::new(vec![1, 2, 1, 2, 1])),
                ),
                (
                    "v".into(),
                    Column::F64(ColumnData::new(vec![1.0, 10.0, 2.0, 20.0, 3.0])),
                ),
            ],
        ));
        let out = Aggregate::new("k", "v", AggOp::Sum, "total")
            .run(&[ds], &Selection::new())
            .unwrap();
        assert_eq!(out_f64(&out, "k"), vec![1.0, 2.0]);
        assert_eq!(out_f64(&out, "total"), vec![6.0, 30.0]);
    }

    #[test]
    fn fingerprint_distinguishes_op_and_columns() {
        let s = Aggregate::new("g", "v", AggOp::Sum, "o").fingerprint();
        let m = Aggregate::new("g", "v", AggOp::Mean, "o").fingerprint();
        let g2 = Aggregate::new("h", "v", AggOp::Sum, "o").fingerprint();
        let v2 = Aggregate::new("g", "w", AggOp::Sum, "o").fingerprint();
        let o2 = Aggregate::new("g", "v", AggOp::Sum, "p").fingerprint();
        for other in [m, g2, v2, o2] {
            assert_ne!(s, other);
        }
    }
}
