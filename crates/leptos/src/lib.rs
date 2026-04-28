//! # `berthacharts-leptos`
//!
//! First-class Leptos bindings. The differentiator: other WebGL chart
//! libraries treat Leptos as an afterthought. This binding is signal-native
//! and ships no JavaScript runtime of its own.
//!
//! v0.1 ships the crate slot. The `<Chart>` component lands in v0.1.1.

#![forbid(unsafe_code)]

pub use berthacharts_core as core;
pub use berthacharts_renderer_wgpu as renderer;
