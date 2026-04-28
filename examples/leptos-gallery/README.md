# Bertha Charts — Leptos Gallery

Living reference for the library as it grows. Each chart type gets an example
component; when you build a new `Mark` / `Scale` / `Transform`, add a section
here so you (and future users) can see and read it.

## Run locally

Prereq: [`trunk`](https://trunkrs.dev/) and the `wasm32-unknown-unknown` Rust
target.

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk                       # one time
cd examples/leptos-gallery
trunk serve                               # http://127.0.0.1:8787
```

## Structure

- `src/main.rs` — mounts the Leptos app.
- `src/app.rs` — top-level shell + nav.
- `src/chart_canvas.rs` — reusable `<ChartCanvas>` component that bridges
  Leptos's `NodeRef<Canvas>` to `berthacharts_renderer_wgpu::Renderer`.
- `src/examples/<name>.rs` — one file per demo.

## Quality gates

This gallery should be treated as both demo and regression surface. A chart is
not considered solid until it passes:

- Rust unit tests for layout invariants, such as Sankey stack conservation.
- `cargo test --workspace` from the repository root.
- `cargo test --manifest-path examples/leptos-gallery/Cargo.toml` for gallery
  logic that is deliberately outside the main workspace.
- `trunk build` from this directory for the wasm/WebGL2 target.
- Browser screenshots in Chromium, Firefox, and WebKit/Safari at desktop,
  tablet, and mobile widths, including DPR 1, 2, and 3 when available.

The current gallery uses fixed logical chart sizes with horizontal overflow as
the fallback at narrow widths. The next responsive step is container-measured
layout: chart specs should receive the available width, recompute scales and
guide budgets, then redraw the canvas and overlay together.

## Adding a new example

1. New file in `src/examples/`, e.g. `scatter.rs`.
2. Export a `#[component] pub fn View()` that builds a chart and wraps a
   `<ChartCanvas>`.
3. Add `pub mod scatter;` to `src/examples/mod.rs`.
4. Add `<examples::scatter::View />` to `App` in `src/app.rs`.

## Intentional limitations (v0.1)

- One render on mount; no signal-driven redraw yet. Pan / zoom / interactive
  brushing land with the reactive binding in `berthacharts-leptos`.
- `ColorChannel::Column` renders transparent until color-scale support ships.
- Axis labels, legends, data labels, and tooltips are DOM/SVG overlays. They
  are intentionally separate from the GPU mark layer so text stays readable,
  but label placement still relies on estimated sizes rather than measured
  browser boxes.

## Troubleshooting

- **Blank canvas / WebGL errors in console.** Confirm your browser has WebGL2.
  Safari: Develop → Experimental Features → WebGL 2.0. Chrome/Firefox: on by
  default.
- **`wgpu adapter` error.** Almost always WebGL disabled or a driver mismatch.
