# Bertha Charts

A WebGL chart kernel for Rust and the web. Foundational primitives for
sophisticated, analytically-deep data visualization — consumable from React
(via WASM) and Leptos (natively).

**Status:** v0.0.1 — pre-release. Core kernel, wgpu renderer, and a starter
set of marks are implemented; domain crates (transforms / stats / dist /
finance / anno) and the React/Leptos binding crates are stubs reserving the
public surface and will land in v0.1.x. Public traits are not yet stable.

## Architecture

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
```

The Leptos gallery is built with [Trunk](https://trunkrs.dev/):

```sh
cd examples/leptos-gallery && trunk serve
```

## License

MIT OR Apache-2.0
