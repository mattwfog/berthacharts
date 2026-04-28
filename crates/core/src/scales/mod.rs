//! Built-in [`Scale`](crate::Scale) implementations.
//!
//! These ship alongside the trait so the common cases don't require pulling
//! in a separate crate. Custom scales still implement [`Scale`](crate::Scale)
//! on equal footing.

mod band;
mod linear;
mod util;

pub use band::BandScale;
pub use linear::LinearScale;
