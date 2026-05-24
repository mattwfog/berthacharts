//! # `berthacharts-leptos`
//!
//! First-class Leptos bindings. The differentiator: other WebGL chart
//! libraries treat Leptos as an afterthought. This binding is signal-native
//! and ships no JavaScript runtime of its own.
//!
//! ## Usage
//!
//! ```ignore
//! use std::sync::Arc;
//! use berthacharts_leptos::{BuildChart, ChartCanvas};
//! use berthacharts_core::{Chart, Workspace};
//!
//! let build: BuildChart = Arc::new(|ws: Arc<Workspace>| -> Chart {
//!     // populate scales, marks, etc. on the workspace and return a Chart
//!     todo!()
//! });
//!
//! view! { <ChartCanvas width=480 height=320 builder=build /> }
//! ```
//!
//! For richer compositions (overlays, toolbars, custom layout) use
//! [`mount_renderer`] directly with your own `<canvas>` element.

#![forbid(unsafe_code)]

pub use berthacharts_core as core;
pub use berthacharts_renderer_wgpu as renderer;

mod chart;
pub mod interaction;

pub use chart::{browser_device_pixel_ratio, mount_renderer, physical_px, BuildChart, ChartCanvas};
pub use interaction::{
    interpolate_color, interpolate_f32, interpolate_point, use_drag, use_tween, DragPhase,
    DragState, Easing,
};
