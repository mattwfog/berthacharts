//! Cumulative stacking within partitions.
//!
//! For each partition (defined by `partition`), iterate rows in their input
//! order and emit two new columns:
//!
//! * `output_lo` — the cumulative running total **before** this row's value
//! * `output_hi` — the cumulative running total **after** this row's value
//!
//! This is the canonical primitive for stacked bars / areas: pair the result
//! with a `y` scale on `output_lo`/`output_hi` to draw stacks.
//!
//! v0.1 supports zero-baseline stacking only. `Center` (streamgraph) and
//! `Normalize` (100% stacking) modes will land alongside the same row-iteration
//! machinery — they are different reductions over the same partition iterator.

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Arc;

use berthacharts_core::{
    Column, ColumnData, Dataset, Selection, Transform, TransformError, TransformInputs,
    TransformOutput,
};

use crate::fingerprint::{derive_output_dataset_id, Hasher};

const NAME: &str = "stack";

/// Stack a value column within a partition column, producing `lo`/`hi` columns.
#[derive(Debug, Clone)]
pub struct Stack {
    /// Column whose values define partitions. Each partition is stacked
    /// independently (so e.g. a per-x-tick categorical bar partition stacks
    /// independently of its neighbors).
    pub partition: String,
    /// Column to stack. Must be numeric.
    pub value: String,
    /// Output column for the running total before this row's value.
    pub output_lo: String,
    /// Output column for the running total after this row's value.
    pub output_hi: String,
}

impl Stack {
    /// Construct a new stacker.
    #[must_use]
    pub fn new(
        partition: impl Into<String>,
        value: impl Into<String>,
        output_lo: impl Into<String>,
        output_hi: impl Into<String>,
    ) -> Self {
        Self {
            partition: partition.into(),
            value: value.into(),
            output_lo: output_lo.into(),
            output_hi: output_hi.into(),
        }
    }
}

impl Transform for Stack {
    fn name(&self) -> &'static str {
        NAME
    }

    fn fingerprint(&self) -> u64 {
        let mut h = Hasher::new();
        h.write_str(NAME)
            .write_str(&self.partition)
            .write_str(&self.value)
            .write_str(&self.output_lo)
            .write_str(&self.output_hi);
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
        let partition = input
            .column(&self.partition)
            .ok_or_else(|| TransformError::MissingColumn(self.partition.clone()))?;
        let value = input
            .column(&self.value)
            .ok_or_else(|| TransformError::MissingColumn(self.value.clone()))?;

        // Running total per partition. Keyed by a row-stable key so the same
        // string / number values stack together.
        let mut totals: BTreeMap<PartitionKey, f64> = BTreeMap::new();
        let mut lo_col = Vec::with_capacity(input.len);
        let mut hi_col = Vec::with_capacity(input.len);

        for row in 0..input.len {
            let key = PartitionKey::for_row(partition, row);
            let v = value.read_f64(row).filter(|x| x.is_finite()).unwrap_or(0.0);
            let entry = totals.entry(key).or_insert(0.0);
            let lo = *entry;
            let hi = lo + v;
            *entry = hi;
            lo_col.push(lo);
            hi_col.push(hi);
        }

        let mut columns: Vec<(String, Column)> = input
            .columns
            .iter()
            .map(|(k, v)| (k.clone(), (**v).clone()))
            .collect();
        columns.push((self.output_lo.clone(), Column::F64(ColumnData::new(lo_col))));
        columns.push((self.output_hi.clone(), Column::F64(ColumnData::new(hi_col))));

        let next_id = derive_output_dataset_id(self.fingerprint(), input.id);
        let next_version = input.version.wrapping_add(1);
        Ok(Arc::new(Dataset::new(next_id, next_version, columns)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Stable per-row partition key. We only need equality + ordering, not the
/// original value back, so we encode every kind into a comparable byte form.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PartitionKey {
    Str(String),
    Bits(u64),
    None,
}

impl PartitionKey {
    fn for_row(col: &Arc<Column>, row: usize) -> Self {
        match col.as_ref() {
            Column::Utf8(d) => d
                .values
                .get(row)
                .map(|s| PartitionKey::Str(s.to_string()))
                .unwrap_or(PartitionKey::None),
            _ => col
                .read_f64(row)
                .filter(|x| x.is_finite())
                .map(|v| PartitionKey::Bits(v.to_bits()))
                .unwrap_or(PartitionKey::None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Column, ColumnData, DatasetId};

    fn fixture() -> Arc<Dataset> {
        // partition x: A A B A B
        // value y:     1 2 3 4 5
        // expected stacks (per partition, in input order):
        //   row 0 (A, 1) → [0, 1]
        //   row 1 (A, 2) → [1, 3]
        //   row 2 (B, 3) → [0, 3]
        //   row 3 (A, 4) → [3, 7]
        //   row 4 (B, 5) → [3, 8]
        Arc::new(Dataset::new(
            DatasetId::new(20),
            0,
            vec![
                (
                    "x".into(),
                    Column::Utf8(ColumnData::new(vec![
                        Arc::from("A"),
                        Arc::from("A"),
                        Arc::from("B"),
                        Arc::from("A"),
                        Arc::from("B"),
                    ])),
                ),
                (
                    "y".into(),
                    Column::F64(ColumnData::new(vec![1.0, 2.0, 3.0, 4.0, 5.0])),
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

    #[test]
    fn stacks_within_partition_in_input_order() {
        let s = Stack::new("x", "y", "lo", "hi");
        let out = s.run(&[fixture()], &Selection::new()).unwrap();
        assert_eq!(out_f64(&out, "lo"), vec![0.0, 1.0, 0.0, 3.0, 3.0]);
        assert_eq!(out_f64(&out, "hi"), vec![1.0, 3.0, 3.0, 7.0, 8.0]);
    }

    #[test]
    fn preserves_other_columns() {
        let s = Stack::new("x", "y", "lo", "hi");
        let out = s.run(&[fixture()], &Selection::new()).unwrap();
        assert!(out.column("x").is_some());
        assert!(out.column("y").is_some());
        assert_eq!(out.len, 5);
    }

    #[test]
    fn nonfinite_values_treated_as_zero() {
        let ds = Arc::new(Dataset::new(
            DatasetId::new(21),
            0,
            vec![
                (
                    "p".into(),
                    Column::Utf8(ColumnData::new(vec![
                        Arc::from("a"),
                        Arc::from("a"),
                        Arc::from("a"),
                    ])),
                ),
                (
                    "v".into(),
                    Column::F64(ColumnData::new(vec![1.0, f64::NAN, 2.0])),
                ),
            ],
        ));
        let out = Stack::new("p", "v", "lo", "hi")
            .run(&[ds], &Selection::new())
            .unwrap();
        assert_eq!(out_f64(&out, "lo"), vec![0.0, 1.0, 1.0]);
        assert_eq!(out_f64(&out, "hi"), vec![1.0, 1.0, 3.0]);
    }

    #[test]
    fn missing_partition_column_errors() {
        let err = Stack::new("nope", "y", "lo", "hi")
            .run(&[fixture()], &Selection::new())
            .unwrap_err();
        assert!(matches!(err, TransformError::MissingColumn(c) if c == "nope"));
    }

    #[test]
    fn missing_value_column_errors() {
        let err = Stack::new("x", "nope", "lo", "hi")
            .run(&[fixture()], &Selection::new())
            .unwrap_err();
        assert!(matches!(err, TransformError::MissingColumn(c) if c == "nope"));
    }
}
