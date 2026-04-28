//! Scale trait: domain (f64) → range (f32) projection.
//!
//! Scales are dyn-safe. Built-in implementations (`LinearScale`, `LogScale`,
//! `TimeScale`, `BandScale`, `OrdinalScale`) live outside the kernel so users
//! can provide custom scales on equal footing.

use std::fmt::Debug;
use std::sync::Arc;

use ahash::AHashMap;

pub use crate::ids::ScaleId;

/// A scale projects values from a domain (conceptually f64 for precision) to
/// a visual range (f32 pixels or normalized device coords).
///
/// Implementations MUST be deterministic: identical state ⇒ identical
/// `project`, `ticks`, `fingerprint`, `gpu_uniforms` outputs.
pub trait Scale: Debug + Send + Sync + 'static {
    /// Project a single domain value to its range position. Values outside the
    /// domain may be extrapolated unless the implementation documents otherwise.
    fn project(&self, value: f64) -> f32;

    /// Inverse projection: given a range position, find the domain value.
    /// Returns `None` for scales without a well-defined inverse (e.g. ordinal).
    fn unproject(&self, position: f32) -> Option<f64>;

    /// Generate ticks suitable for axis / gridline rendering.
    ///
    /// `count` is a hint; implementations may return fewer or more based on
    /// "nice" tick selection.
    fn ticks(&self, count: usize) -> Vec<Tick>;

    /// 64-bit cache key. MUST change whenever the scale's visible behavior
    /// would change; MAY remain stable across semantically-identical instances.
    fn fingerprint(&self) -> u64;

    /// GPU-uploadable summary used by shaders that project per-vertex.
    fn gpu_uniforms(&self) -> ScaleUniforms;

    /// Type-erased downcast support. Implementers can return `self`.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// A single axis tick position and label.
#[derive(Debug, Clone)]
pub struct Tick {
    /// Domain-space value of the tick.
    pub value: f64,
    /// Pre-projected range-space position (for overlay rendering convenience).
    pub position: f32,
    /// Human-readable label.
    pub label: String,
}

/// Scale parameters packed for GPU consumption.
///
/// The layout is fixed at 48 bytes so shaders can `std140`/`std430` bind it
/// uniformly across scale kinds. `kind` discriminates how the remaining fields
/// are interpreted.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleUniforms {
    /// Discriminator: 0=linear, 1=log, 2=time, 3=band, 4=ordinal, 5=custom.
    pub kind: u32,
    /// Bit flags (clamp=1, nice=2, reversed=4, ...).
    pub flags: u32,
    /// Low end of the domain (f32 truncation; see `domain_lo_hi`).
    pub domain_lo: f32,
    /// High end of the domain (f32 truncation).
    pub domain_hi: f32,
    /// Low end of the range (pixels or NDC).
    pub range_lo: f32,
    /// High end of the range.
    pub range_hi: f32,
    /// High-precision delta for `domain_lo` — classic deck.gl split-f32 trick.
    pub domain_lo_hi: f32,
    /// High-precision delta for `domain_hi`.
    pub domain_hi_hi: f32,
    /// Auxiliary scalar — log-base for log scales, step size for band scales.
    pub aux0: f32,
    /// Second auxiliary scalar — padding_inner for band, unused otherwise.
    pub aux1: f32,
    /// Reserved.
    pub _pad0: f32,
    /// Reserved.
    pub _pad1: f32,
}

impl Default for ScaleUniforms {
    fn default() -> Self {
        Self {
            kind: 0,
            flags: 0,
            domain_lo: 0.0,
            domain_hi: 1.0,
            range_lo: 0.0,
            range_hi: 1.0,
            domain_lo_hi: 0.0,
            domain_hi_hi: 0.0,
            aux0: 0.0,
            aux1: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

/// Registry of scales owned by a [`crate::Workspace`].
#[derive(Debug, Default, Clone)]
pub struct ScaleRegistry {
    scales: AHashMap<ScaleId, Arc<dyn Scale>>,
}

impl ScaleRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a scale.
    pub fn upsert(&mut self, id: ScaleId, scale: Arc<dyn Scale>) -> Option<Arc<dyn Scale>> {
        self.scales.insert(id, scale)
    }

    /// Remove a scale by id.
    pub fn remove(&mut self, id: ScaleId) -> Option<Arc<dyn Scale>> {
        self.scales.remove(&id)
    }

    /// Borrow a scale by id.
    #[must_use]
    pub fn get(&self, id: ScaleId) -> Option<&Arc<dyn Scale>> {
        self.scales.get(&id)
    }

    /// Iterate.
    pub fn iter(&self) -> impl Iterator<Item = (&ScaleId, &Arc<dyn Scale>)> {
        self.scales.iter()
    }

    /// Combined fingerprint of all registered scales.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut pairs: Vec<_> = self
            .scales
            .iter()
            .map(|(k, v)| (*k, v.fingerprint()))
            .collect();
        pairs.sort_unstable_by_key(|(k, _)| *k);
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for (k, fp) in pairs {
            hash ^= u64::from(k.get());
            hash = hash.wrapping_mul(0x0100_0000_01b3);
            hash ^= fp;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash
    }
}
