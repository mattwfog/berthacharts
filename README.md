# Bertha Charts

A WebGL chart kernel for Rust and the web. Foundational primitives for
sophisticated, analytically-deep data visualization, consumable from Rust,
Leptos, and React applications.

**Status:** v0.0.2 — pre-release. Core kernel, wgpu renderer, starter chart
specs, Rust facade crate, Leptos bindings, and public React/WASM npm bindings
are implemented. Public Rust traits are not yet stable.

## Quick Start

### Rust

Use the facade crate for Rust application code:

```toml
[dependencies]
berthacharts = "0.0.2"
```

When working from this repository instead of crates.io, use the local facade:

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

### React

Install the npm package:

```sh
npm install @berthacharts/react
```

Render a chart with typed React props. The React layer owns canvas lifecycle,
WASM initialization, resize handling, cleanup, and the DOM guide overlay.

```tsx
import { BarChart } from "@berthacharts/react";

export function RevenueChart() {
  return (
    <BarChart
      data={[
        { label: "Q1", value: 24 },
        { label: "Q2", value: 31 },
        { label: "Q3", value: 37 },
      ]}
      options={{ target: 30, xLabel: "Quarter", yLabel: "Revenue" }}
      style={{ height: 320 }}
    />
  );
}
```

Advanced users can import the raw wasm-pack API from the `./wasm` export:

```ts
import initWasm, { BerthaChart } from "@berthacharts/react/wasm";
```

Default features include `charts` and `transforms`. Optional feature flags for
the initial public release are `stats`, `geo`, `network`, `renderer-wgpu`, and
`leptos`.

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
- **Domain crates** (`transforms`, `stats`, `geo`, `network`) — opt-in domain
  logic built on the core trait surface.
- **Renderer** (`renderer-wgpu`) — wgpu backend targeting WebGL2 on the web
  and native elsewhere.
- **Bindings** — `berthacharts-leptos` provides Rust-native Leptos bindings.
  `@berthacharts/react` provides npm React components plus the raw WASM API.

Incubating crates for annotations, distribution marks, and finance charts are
kept private in this repository until they have usable public APIs.

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
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test -p berthacharts
cargo test -p berthacharts --features network,geo,stats
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
scripts/build-npm.sh
(cd crates/bindings-react/pkg && npm --cache /private/tmp/bertha-npm-cache pack --dry-run)
node scripts/check-npm-react.mjs crates/bindings-react/pkg
```

`cargo package -p berthacharts` requires the leaf crates
(`berthacharts-core`, `berthacharts-charts`, `berthacharts-transforms`,
`berthacharts-stats`, `berthacharts-geo`, `berthacharts-network`,
`berthacharts-renderer-wgpu`, and `berthacharts-leptos`) to be published
first, because Cargo resolves facade dependencies through the registry during
package verification. See `RELEASE.md` for the publish order.

The Leptos gallery is built with [Trunk](https://trunkrs.dev/):

```sh
cd examples/leptos-gallery && trunk serve
```

## License

MIT OR Apache-2.0
