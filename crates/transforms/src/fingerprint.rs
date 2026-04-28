//! Tiny FNV-1a hasher used for transform fingerprints.
//!
//! Transforms must produce a stable 64-bit cache key that changes whenever any
//! configured parameter would change their output. FNV-1a is dependency-free,
//! deterministic across runs, and good enough as a cache key (collisions are
//! survivable — the DAG will simply recompute and overwrite). The same constants
//! are used in `core::Selection::fingerprint`, keeping fingerprint style
//! consistent across the workspace.
//!
//! This is `pub(crate)` — fingerprint construction is an implementation detail
//! of each transform.
//!
//! # Example
//! ```ignore
//! let fp = Hasher::new()
//!     .write_str("filter_range")
//!     .write_str(&self.column)
//!     .write_f64(self.range.start)
//!     .write_f64(self.range.end)
//!     .finish();
//! ```
const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0100_0000_01b3;

pub(crate) struct Hasher(u64);

impl Hasher {
    pub(crate) fn new() -> Self {
        Self(OFFSET)
    }

    pub(crate) fn write_u64(&mut self, v: u64) -> &mut Self {
        self.0 ^= v;
        self.0 = self.0.wrapping_mul(PRIME);
        self
    }

    pub(crate) fn write_usize(&mut self, v: usize) -> &mut Self {
        self.write_u64(v as u64)
    }

    pub(crate) fn write_f64(&mut self, v: f64) -> &mut Self {
        // Hash bit-pattern so NaN / +0 / -0 distinctions affect the fingerprint
        // (different bits ⇒ potentially different output).
        self.write_u64(v.to_bits())
    }

    pub(crate) fn write_str(&mut self, s: &str) -> &mut Self {
        for b in s.bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(PRIME);
        }
        self
    }

    pub(crate) fn finish(self) -> u64 {
        self.0
    }
}

/// Derive a stable [`DatasetId`](berthacharts_core::DatasetId) for a transform's
/// output, given the transform's fingerprint and the (single) input dataset's
/// id. Same inputs ⇒ same output id; different inputs / different transform
/// configuration ⇒ different output id.
pub(crate) fn derive_output_dataset_id(
    transform_fp: u64,
    input_id: berthacharts_core::DatasetId,
) -> berthacharts_core::DatasetId {
    let mut h = Hasher::new();
    h.write_u64(transform_fp)
        .write_u64(u64::from(input_id.get()));
    let raw = h.finish();
    // DatasetId is u32 — fold the 64-bit fingerprint into 32 bits.
    let folded = ((raw >> 32) ^ raw) as u32;
    berthacharts_core::DatasetId::new(folded)
}
