//! Equal-width binning over a numeric column.
//!
//! Adds an `i64` column (named via `output_index`) holding the bin index
//! `[0, n_bins)` for each row. Rows whose value falls exactly on the upper
//! edge are placed in the last bin (closed on the right at the very end so
//! every input row gets a bin). Non-finite or non-numeric values are encoded
//! as `-1` — callers can filter those out with [`crate::FilterRange`] or
//! handle them downstream.
//!
//! v0.1 ships only equal-width bins. Quantile / log binning land later.

use std::any::Any;
use std::sync::Arc;

use berthacharts_core::{
    Column, ColumnData, Dataset, Selection, Transform, TransformError, TransformInputs,
    TransformOutput,
};

use crate::fingerprint::{derive_output_dataset_id, Hasher};

const NAME: &str = "bin";

/// Add a bin-index column to a dataset.
#[derive(Debug, Clone)]
pub struct Bin {
    /// Column to bin over.
    pub value: String,
    /// Number of equal-width bins. Must be `>= 1`.
    pub n_bins: usize,
    /// Name of the output bin-index column to add.
    pub output_index: String,
}

impl Bin {
    /// Construct a new equal-width binner.
    #[must_use]
    pub fn new(value: impl Into<String>, n_bins: usize, output_index: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            n_bins,
            output_index: output_index.into(),
        }
    }
}

impl Transform for Bin {
    fn name(&self) -> &'static str {
        NAME
    }

    fn fingerprint(&self) -> u64 {
        let mut h = Hasher::new();
        h.write_str(NAME)
            .write_str(&self.value)
            .write_usize(self.n_bins)
            .write_str(&self.output_index);
        h.finish()
    }

    fn run(
        &self,
        inputs: TransformInputs<'_>,
        _selection: &Selection,
    ) -> Result<TransformOutput, TransformError> {
        if self.n_bins == 0 {
            return Err(TransformError::Other {
                name: NAME,
                message: "n_bins must be >= 1".into(),
            });
        }
        if inputs.len() != 1 {
            return Err(TransformError::Arity {
                expected: 1,
                actual: inputs.len(),
            });
        }
        let input = &inputs[0];
        let value_col = input
            .column(&self.value)
            .ok_or_else(|| TransformError::MissingColumn(self.value.clone()))?;

        let (lo, hi) = column_finite_range(value_col).unwrap_or((0.0, 0.0));
        let span = hi - lo;
        let bin_indices: Vec<i64> = (0..input.len)
            .map(|row| match value_col.read_f64(row) {
                Some(v) if v.is_finite() => {
                    if span == 0.0 {
                        0
                    } else {
                        let frac = (v - lo) / span;
                        let idx = (frac * self.n_bins as f64).floor() as i64;
                        idx.clamp(0, self.n_bins as i64 - 1)
                    }
                }
                _ => -1,
            })
            .collect();

        let mut columns: Vec<(String, Column)> = input
            .columns
            .iter()
            .map(|(k, v)| (k.clone(), (**v).clone()))
            .collect();
        columns.push((
            self.output_index.clone(),
            Column::I64(ColumnData::new(bin_indices)),
        ));

        let next_id = derive_output_dataset_id(self.fingerprint(), input.id);
        let next_version = input.version.wrapping_add(1);
        Ok(Arc::new(Dataset::new(next_id, next_version, columns)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn column_finite_range(col: &Arc<Column>) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut any = false;
    for row in 0..col.len() {
        if let Some(v) = col.read_f64(row) {
            if v.is_finite() {
                any = true;
                if v < lo {
                    lo = v;
                }
                if v > hi {
                    hi = v;
                }
            }
        }
    }
    if any {
        Some((lo, hi))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Column, ColumnData, DatasetId};

    fn fixture() -> Arc<Dataset> {
        Arc::new(Dataset::new(
            DatasetId::new(2),
            0,
            vec![(
                "x".into(),
                Column::F64(ColumnData::new(vec![0.0, 2.5, 5.0, 7.5, 10.0])),
            )],
        ))
    }

    fn out_i64(ds: &Dataset, name: &str) -> Vec<i64> {
        match ds.column(name).unwrap().as_ref() {
            Column::I64(d) => d.values.clone(),
            _ => panic!("expected i64 column"),
        }
    }

    #[test]
    fn equal_width_bins_partition_the_range() {
        // 0..10 split into 4 bins ⇒ widths 2.5: [0,2.5), [2.5,5.0), [5.0,7.5), [7.5,10.0]
        let b = Bin::new("x", 4, "bin");
        let out = b.run(&[fixture()], &Selection::new()).unwrap();
        assert_eq!(out_i64(&out, "bin"), vec![0, 1, 2, 3, 3]);
    }

    #[test]
    fn single_bin_collapses_to_zero() {
        let b = Bin::new("x", 1, "bin");
        let out = b.run(&[fixture()], &Selection::new()).unwrap();
        assert_eq!(out_i64(&out, "bin"), vec![0; 5]);
    }

    #[test]
    fn zero_span_uses_bin_zero() {
        let ds = Arc::new(Dataset::new(
            DatasetId::new(3),
            0,
            vec![(
                "x".into(),
                Column::F64(ColumnData::new(vec![1.0, 1.0, 1.0])),
            )],
        ));
        let out = Bin::new("x", 5, "bin")
            .run(&[ds], &Selection::new())
            .unwrap();
        assert_eq!(out_i64(&out, "bin"), vec![0, 0, 0]);
    }

    #[test]
    fn non_finite_values_get_minus_one() {
        let ds = Arc::new(Dataset::new(
            DatasetId::new(4),
            0,
            vec![(
                "x".into(),
                Column::F64(ColumnData::new(vec![0.0, f64::NAN, 5.0, f64::INFINITY])),
            )],
        ));
        let out = Bin::new("x", 2, "bin")
            .run(&[ds], &Selection::new())
            .unwrap();
        let bins = out_i64(&out, "bin");
        assert_eq!(bins[0], 0);
        assert_eq!(bins[1], -1);
        assert_eq!(bins[2], 1);
        assert_eq!(bins[3], -1);
    }

    #[test]
    fn n_bins_zero_errors() {
        let err = Bin::new("x", 0, "bin")
            .run(&[fixture()], &Selection::new())
            .unwrap_err();
        assert!(matches!(err, TransformError::Other { name, .. } if name == "bin"));
    }

    #[test]
    fn fingerprint_changes_on_config_change() {
        let a = Bin::new("x", 4, "bin").fingerprint();
        let b = Bin::new("x", 5, "bin").fingerprint();
        let c = Bin::new("x", 4, "other").fingerprint();
        let d = Bin::new("y", 4, "bin").fingerprint();
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }
}
