//! Visual channels: how a mark property is produced (constant or data-driven).
//!
//! A `Channel` binds a mark attribute (x, y, stroke, width, ...) to either a
//! scalar constant or to a column of a dataset through a scale.

use crate::ids::{DatasetId, ScaleId};

fn hash_f32(h: &mut u64, v: f32) {
    *h ^= u64::from(v.to_bits());
    *h = h.wrapping_mul(0x0100_0000_01b3);
}

fn hash_u32(h: &mut u64, v: u32) {
    *h ^= u64::from(v);
    *h = h.wrapping_mul(0x0100_0000_01b3);
}

fn hash_str(h: &mut u64, v: &str) {
    for byte in v.as_bytes() {
        *h ^= u64::from(*byte);
        *h = h.wrapping_mul(0x0100_0000_01b3);
    }
    *h ^= 0xff;
    *h = h.wrapping_mul(0x0100_0000_01b3);
}

/// Numeric visual channel (position, size, width, opacity).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NumberChannel {
    /// Fixed scalar applied to all rows.
    Constant(f32),
    /// Column values projected through a scale.
    Column {
        /// Dataset providing the column.
        dataset: DatasetId,
        /// Column name.
        name: String,
        /// Scale used to project domain → range.
        scale: ScaleId,
    },
    /// Numeric channel plus a constant offset after the base channel resolves.
    Offset {
        /// Base channel.
        base: Box<NumberChannel>,
        /// Constant offset in the channel's resolved units.
        offset: f32,
    },
}

impl NumberChannel {
    /// Wrap this channel with a constant offset.
    #[must_use]
    pub fn offset(self, offset: f32) -> Self {
        Self::Offset {
            base: Box::new(self),
            offset,
        }
    }

    /// Stable cache key for the channel configuration.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        self.hash_into(&mut h);
        h
    }

    pub(crate) fn hash_into(&self, h: &mut u64) {
        match self {
            Self::Constant(v) => {
                hash_u32(h, 1);
                hash_f32(h, *v);
            }
            Self::Column {
                dataset,
                name,
                scale,
            } => {
                hash_u32(h, 2);
                hash_u32(h, dataset.get());
                hash_str(h, name);
                hash_u32(h, scale.get());
            }
            Self::Offset { base, offset } => {
                hash_u32(h, 3);
                base.hash_into(h);
                hash_f32(h, *offset);
            }
        }
    }
}

/// Color channel. Colors resolve to pre-multiplied RGBA at upload time.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ColorChannel {
    /// Fixed RGBA (0–1 per component, pre-multiplied alpha expected).
    Constant([f32; 4]),
    /// Column values projected through a color scale.
    ///
    /// The referenced `scale` must be a color-producing scale (its
    /// [`crate::Scale::project`] return value is interpreted as a packed
    /// color index into a palette shader texture).
    Column {
        /// Dataset providing the column.
        dataset: DatasetId,
        /// Column name.
        name: String,
        /// Color scale (e.g. quantized, diverging, ordinal palette).
        scale: ScaleId,
    },
    /// RGBA components read directly from numeric columns.
    RgbaColumns {
        /// Dataset providing the columns.
        dataset: DatasetId,
        /// Red component column.
        r: String,
        /// Green component column.
        g: String,
        /// Blue component column.
        b: String,
        /// Optional alpha component column. Missing means fully opaque.
        a: Option<String>,
    },
}

impl ColorChannel {
    /// Stable cache key for the channel configuration.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        self.hash_into(&mut h);
        h
    }

    pub(crate) fn hash_into(&self, h: &mut u64) {
        match self {
            Self::Constant(c) => {
                hash_u32(h, 1);
                for component in c {
                    hash_f32(h, *component);
                }
            }
            Self::Column {
                dataset,
                name,
                scale,
            } => {
                hash_u32(h, 2);
                hash_u32(h, dataset.get());
                hash_str(h, name);
                hash_u32(h, scale.get());
            }
            Self::RgbaColumns {
                dataset,
                r,
                g,
                b,
                a,
            } => {
                hash_u32(h, 3);
                hash_u32(h, dataset.get());
                hash_str(h, r);
                hash_str(h, g);
                hash_str(h, b);
                if let Some(a) = a {
                    hash_str(h, a);
                }
            }
        }
    }
}

/// Generic alias used in docstrings where either kind can appear.
#[derive(Debug, Clone)]
pub enum Channel {
    /// A numeric channel.
    Number(NumberChannel),
    /// A color channel.
    Color(ColorChannel),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_channel_fingerprint_tracks_offset() {
        let base = NumberChannel::Column {
            dataset: DatasetId::new(1),
            name: "x".into(),
            scale: ScaleId::new(2),
        };

        assert_ne!(
            base.clone().offset(10.0).fingerprint(),
            base.offset(11.0).fingerprint()
        );
    }

    #[test]
    fn color_channel_fingerprint_tracks_constant_color() {
        let a = ColorChannel::Constant([1.0, 0.0, 0.0, 1.0]);
        let b = ColorChannel::Constant([0.0, 1.0, 0.0, 1.0]);

        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn color_channel_fingerprint_tracks_rgba_columns() {
        let a = ColorChannel::RgbaColumns {
            dataset: DatasetId::new(1),
            r: "r".into(),
            g: "g".into(),
            b: "b".into(),
            a: None,
        };
        let b = ColorChannel::RgbaColumns {
            dataset: DatasetId::new(2),
            r: "r".into(),
            g: "g".into(),
            b: "b".into(),
            a: None,
        };

        assert_ne!(a.fingerprint(), b.fingerprint());
    }
}
