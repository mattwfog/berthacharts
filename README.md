# Bertha Charts

A WebGL chart kernel for Rust and the web. Foundational primitives for
sophisticated, analytically-deep data visualization — consumable from React
(via WASM) and Leptos (natively).

**Status:** v0.0.1 — pre-release. Core kernel, wgpu renderer, and a starter
set of reusable chart specs are implemented. Public traits are not yet stable.

## Quick Start

Use the facade crate for application code:

```toml
[dependencies]
berthacharts = { path = "crates/berthacharts" }
```

Then build chart specs through the prelude:

```rust
use berthacharts::prelude::*;

let spec = BarChartSpec::new(vec![
    BarDatum::new("Q1", 24.0),
    BarDatum::new("Q2", 31.0),
    BarDatum::new("Q3", 37.0),
])
.with_target(30.0);

let chart = spec.build(ChartSize::new(640, 360))?;
assert!(!chart.scene().layers.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Default features include `charts` and `transforms`. Optional feature flags are
available for `stats`, `distribution`, `finance`, `geo`, `network`,
`annotations`, `renderer-wgpu`, `leptos`, and `react`.

Runnable examples:

```sh
cargo run -p berthacharts --example basic
cargo run -p berthacharts --example multiple_charts
cargo run -p berthacharts --features network --example network
```

For coordinated views or shared state, build into an existing workspace:

```rust
use berthacharts::prelude::*;

let workspace = Workspace::new();
let chart = BarChartSpec::new(vec![BarDatum::new("A", 1.0)])
    .build_in(workspace.clone(), ChartSize::new(480, 320))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Architecture

- **Facade** (`berthacharts`) — one import path for application code. Re-exports
  core primitives, chart specs, transforms, and optional domain/rendering
  crates behind feature flags.
- **Core kernel** (`berthacharts-core`) — scales, coordinate systems,
  datasets, scene graph, mark/transform traits, lazy memoized transform DAG,
  picker, event system. No rendering, no framework bindings.
- **Charts** (`berthacharts-charts`) — first-party mark implementations
  (bar, line, scatter, heatmap) built on the core trait surface.
- **Network** (`berthacharts-network`) — graph charts and layouts. Sankey
  ships first; force-directed / chord / tree layouts later.
- **Domain crates** (`transforms`, `stats`, `dist`, `finance`, `anno`) —
  opt-in domain logic. Each implements `Transform` / `Mark` from core. Slots
  reserved at v0.0.1; seed implementations land in v0.1.x.
- **Renderer** (`renderer-wgpu`) — wgpu backend targeting WebGL2 on the web
  and native elsewhere.
- **Bindings** — `berthacharts-leptos` (Rust-native) and
  `berthacharts-bindings-react` (WASM + TS). Slots reserved at v0.0.1.

A working Leptos example wiring core → renderer → DOM lives under
`examples/leptos-gallery/`.

## Invariants

1. Core kernel is framework-agnostic. No `web-sys`, no `wasm-bindgen`, no
   Leptos, no React.
2. Core avoids `std::thread`, `std::time::Instant`, `println!` to stay
   server/embedded-friendly.
3. Public traits (`Scale`, `Coord`, `Mark`, `Transform`) are the semver
   stability contract. Everything else is `#[doc(hidden)]` or gated.
4. Text and axes live in the DOM overlay, never in the renderer.
5. All transforms expose a 64-bit fingerprint; the DAG memoizes outputs keyed
   by (transform fingerprint, input fingerprints).

## Building

```sh
cargo check --workspace
cargo test --workspace
cargo test -p berthacharts
cargo test -p berthacharts --features network,geo,stats
```

`cargo package -p berthacharts` requires the leaf crates
(`berthacharts-core`, `berthacharts-charts`, `berthacharts-transforms`, etc.)
to be published first, because Cargo resolves facade dependencies through the
registry during package verification.

The Leptos gallery is built with [Trunk](https://trunkrs.dev/):

```sh
cd examples/leptos-gallery && trunk serve
```

## License

MIT OR Apache-2.0
