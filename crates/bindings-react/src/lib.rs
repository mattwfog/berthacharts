//! # `berthacharts-bindings-react`
//!
//! WASM bindings published to npm as `@berthacharts/react`. Exposes a thin,
//! idiomatic React surface over the core kernel + wgpu renderer.
//!
//! v0.1 ships the crate slot. The exported JS API (build, feed data, render,
//! subscribe) lands in v0.1.1 alongside the canonical example.

#![forbid(unsafe_code)]

pub use berthacharts_core as core;
pub use berthacharts_renderer_wgpu as renderer;
