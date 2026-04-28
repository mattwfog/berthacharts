//! Range filter: keep rows where `column ∈ [lo, hi)`.
//!
//! The simplest useful filter: a half-open numeric range over a single column.
//! Rows where the column is missing or non-numeric (e.g. `Utf8`) are dropped.
//! Selection-based filters (`FilterBySelection`) will land alongside the
//! Selection plumbing in a follow-up.

use std::any::Any;
use std::ops::Range;
use std::sync::Arc;

use berthacharts_core::{
    Column, ColumnData, Dataset, Selection, Transform, TransformError, TransformInputs,
    TransformOutput,
};

use crate::fingerprint::{derive_output_dataset_id, Hasher};

const NAME: &str = "filter_range";

/// Keep rows where `column` is finite and falls in `[range.start, range.end)`.
#[derive(Debug, Clone)]
pub struct FilterRange {
    /// Column to filter on.
    pub column: String,
    /// Half-open range applied to the column.
    pub range: Range<f64>,
}

impl FilterRange {
    /// Construct a new range filter.
    #[must_use]
    pub fn new(column: impl Into<String>, range: Range<f64>) -> Self {
        Self {
            column: column.into(),
            range,
        }
    }
}

impl Transform for FilterRange {
    fn name(&self) -> &'static str {
        NAME
    }

    fn fingerprint(&self) -> u64 {
        let mut h = Hasher::new();
        h.write_str(NAME)
            .write_str(&self.column)
            .write_f64(self.range.start)
            .write_f64(self.range.end);
        h.finish()
    }

    fn run(
        &self,
        inputs: TransformInputs<'_>,
        _selection: &Selection,
    ) -> Result<TransformOutput, TransformError> {
        let input = expect_single_input(inputs)?;
        let key = input
            .column(&self.column)
            .ok_or_else(|| TransformError::MissingColumn(self.column.clone()))?;

        let keep_mask: Vec<bool> = (0..input.len)
            .map(|row| {
                matches!(
                    key.read_f64(row),
                    Some(v) if v.is_finite() && v >= self.range.start && v < self.range.end
                )
            })
            .collect();

        let columns = input
            .columns
            .iter()
            .map(|(name, col)| (name.clone(), filter_column(col, &keep_mask)))
            .collect::<Vec<_>>();

        let next_id = derive_output_dataset_id(self.fingerprint(), input.id);
        let next_version = input.version.wrapping_add(1);
        Ok(Arc::new(Dataset::new(next_id, next_version, columns)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn expect_single_input<'a>(
    inputs: TransformInputs<'a>,
) -> Result<&'a Arc<Dataset>, TransformError> {
    if inputs.len() != 1 {
        return Err(TransformError::Arity {
            expected: 1,
            actual: inputs.len(),
        });
    }
    Ok(&inputs[0])
}

fn filter_column(col: &Arc<Column>, mask: &[bool]) -> Column {
    fn pick<T: Clone>(values: &[T], mask: &[bool]) -> Vec<T> {
        values
            .iter()
            .zip(mask.iter())
            .filter_map(|(v, &keep)| if keep { Some(v.clone()) } else { None })
            .collect()
    }
    match col.as_ref() {
        Column::F32(d) => Column::F32(ColumnData::new(pick(&d.values, mask))),
        Column::F64(d) => Column::F64(ColumnData::new(pick(&d.values, mask))),
        Column::I64(d) => Column::I64(ColumnData::new(pick(&d.values, mask))),
        Column::U32(d) => Column::U32(ColumnData::new(pick(&d.values, mask))),
        Column::Bool(d) => Column::Bool(ColumnData::new(pick(&d.values, mask))),
        Column::Utf8(d) => Column::Utf8(ColumnData::new(pick(&d.values, mask))),
        // `Column` is `#[non_exhaustive]` — fall back to a clone of the
        // unfiltered column when a future variant is added. Filter behaviour
        // for that variant should be added explicitly above before relying on
        // the fallback.
        _ => (**col).clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Column, ColumnData, DatasetId};

    fn fixture() -> Arc<Dataset> {
        Arc::new(Dataset::new(
            DatasetId::new(1),
            0,
            vec![
                (
                    "x".into(),
                    Column::F64(ColumnData::new(vec![0.0, 1.0, 2.0, 3.0, 4.0])),
                ),
                (
                    "label".into(),
                    Column::Utf8(ColumnData::new(vec![
                        Arc::from("a"),
                        Arc::from("b"),
                        Arc::from("c"),
                        Arc::from("d"),
                        Arc::from("e"),
                    ])),
                ),
            ],
        ))
    }

    fn out_f64(ds: &Dataset, name: &str) -> Vec<f64> {
        let col = ds.column(name).unwrap();
        match col.as_ref() {
            Column::F64(d) => d.values.clone(),
            _ => panic!("expected f64 column"),
        }
    }

    fn out_utf8(ds: &Dataset, name: &str) -> Vec<String> {
        let col = ds.column(name).unwrap();
        match col.as_ref() {
            Column::Utf8(d) => d.values.iter().map(|s| s.to_string()).collect(),
            _ => panic!("expected utf8 column"),
        }
    }

    #[test]
    fn keeps_rows_in_range_and_drops_others() {
        let f = FilterRange::new("x", 1.0..3.0);
        let out = f.run(&[fixture()], &Selection::new()).unwrap();
        assert_eq!(out.len, 2);
        assert_eq!(out_f64(&out, "x"), vec![1.0, 2.0]);
        assert_eq!(out_utf8(&out, "label"), vec!["b", "c"]);
    }

    #[test]
    fn empty_range_yields_empty_output() {
        let f = FilterRange::new("x", 10.0..20.0);
        let out = f.run(&[fixture()], &Selection::new()).unwrap();
        assert_eq!(out.len, 0);
        assert!(out.column("x").is_some());
    }

    #[test]
    fn missing_column_errors() {
        let f = FilterRange::new("nope", 0.0..1.0);
        let err = f.run(&[fixture()], &Selection::new()).unwrap_err();
        assert!(matches!(err, TransformError::MissingColumn(c) if c == "nope"));
    }

    #[test]
    fn arity_mismatch_errors() {
        let f = FilterRange::new("x", 0.0..1.0);
        let err = f.run(&[], &Selection::new()).unwrap_err();
        assert!(matches!(
            err,
            TransformError::Arity {
                expected: 1,
                actual: 0
            }
        ));
    }

    #[test]
    fn fingerprint_changes_on_config_change() {
        let a = FilterRange::new("x", 0.0..1.0).fingerprint();
        let b = FilterRange::new("x", 0.0..2.0).fingerprint();
        let c = FilterRange::new("y", 0.0..1.0).fingerprint();
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let a = FilterRange::new("x", 0.0..1.0).fingerprint();
        let b = FilterRange::new("x", 0.0..1.0).fingerprint();
        assert_eq!(a, b);
    }
}
