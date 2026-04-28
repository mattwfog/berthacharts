//! Transform trait: pure, fingerprinted data → data functions.
//!
//! Transforms form a DAG between raw [`crate::Dataset`]s and the datasets
//! consumed by marks. Each transform is pure (same inputs ⇒ same output) and
//! exposes a fingerprint; the [`crate::Dag`] memoizes results keyed by the
//! joint fingerprint of (transform, inputs).

use std::fmt::Debug;
use std::sync::Arc;

use smallvec::SmallVec;

use crate::dataset::Dataset;
use crate::error::TransformError;
use crate::selection::Selection;

/// Inputs delivered to [`Transform::run`].
///
/// Most transforms take exactly one upstream dataset. Joins / cross-filter /
/// blending transforms take several. A variable-arity transform should
/// validate in its own body.
pub type TransformInputs<'a> = &'a [Arc<Dataset>];

/// Output of a transform. Always a dataset so transforms chain uniformly —
/// KDE returns `{x, density}`, OLS returns `{x, y_hat, lo, hi}`, and so on.
pub type TransformOutput = Arc<Dataset>;

/// Pure, composable data transformation.
///
/// Implementations MUST be deterministic and MUST NOT hold interior mutable
/// state that affects output. The [`Transform::fingerprint`] return value
/// serves as the cache key — any state that influences output must be folded
/// into it.
pub trait Transform: Debug + Send + Sync + 'static {
    /// Static name for logs and profiling.
    fn name(&self) -> &'static str;

    /// Number of upstream datasets expected. Use `0` for source transforms
    /// that synthesize data, `1` for single-input transforms, `>1` for joins.
    fn input_arity(&self) -> usize {
        1
    }

    /// 64-bit cache key. MUST change whenever the transform's output would
    /// change. Include all configured parameters in the fingerprint.
    fn fingerprint(&self) -> u64;

    /// Whether this transform depends on the current [`Selection`]. When
    /// `true`, the DAG invalidates its cached output on selection change.
    fn depends_on_selection(&self) -> bool {
        false
    }

    /// Execute the transform.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError`] on arity mismatch, missing columns, dtype
    /// mismatch, or implementation-specific validation failures.
    fn run(
        &self,
        inputs: TransformInputs<'_>,
        selection: &Selection,
    ) -> Result<TransformOutput, TransformError>;

    /// Type-erased downcast support.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Convenience alias: small-vec of transform fingerprints used by DAG keys.
pub type InputFingerprints = SmallVec<[u64; 4]>;
