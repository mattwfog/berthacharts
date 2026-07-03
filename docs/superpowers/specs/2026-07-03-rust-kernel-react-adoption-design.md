# Rust Kernel and React Adoption Design

## Goal

Ship a focused `0.0.3` readiness pass that improves both Rust kernel confidence
and npm adoption for `@berthacharts/react`.

## Scope

This milestone keeps the Rust rendering/kernel contract intact while making the
npm package feel like a React package. It does not attempt to stabilize every
Rust trait or add new chart types. The shared contract is the existing WASM
boundary: chart inputs are serialized into the Rust chart specs, rendered by the
wgpu renderer, and guide data is returned to the DOM layer.

## Architecture

The React package will add hand-authored JavaScript and TypeScript declaration
files under `crates/bindings-react/js/`. The existing wasm-pack build remains
the source of the Rust/WASM module. The npm build script copies the authored
files into `crates/bindings-react/pkg/` after wasm-pack runs and stamps package
metadata so consumers import React components from `@berthacharts/react` while
advanced users can still import the raw WASM API from `@berthacharts/react/wasm`.

The React layer owns canvas lifecycle, lazy WASM initialization, resize
observation, cleanup, JSON serialization, and a DOM guide overlay. Rust remains
responsible for chart validation, layout, scene generation, rendering, and guide
extraction.

## Public React API

The package root exports:

- `BarChart`
- `LineChart`
- `ScatterPlot`
- `Heatmap`
- `Sankey`
- `BerthaChartCanvas`
- `useBerthaChart`
- `initBerthaCharts`

Each component accepts typed props and renders a transparent canvas plus an
absolute-positioned SVG guide overlay. The low-level chart hook accepts
`type`, `data`, `options`, `width`, `height`, and `wasmUrl`. The hook uses the
existing methods on `BerthaChart`: `bar`, `line`, `scatter`, `heatmap`,
`sankey`, `guides`, `resize`, and `destroy`.

## Rust Kernel Track

The kernel work for this milestone is contract hardening rather than a broad
API redesign:

- Document the current public crate and npm split in the root README.
- Document the Rust feature flags and publish status accurately.
- Add release/package checks for the npm package to CI.
- Keep the Rust workspace tests passing.
- Preserve the raw WASM API for users who need direct access.

## Npm Packaging

The build script must be idempotent. Running `scripts/build-npm.sh` repeatedly
must not duplicate `files` entries. The generated package should include:

- `react.js`
- `react.d.ts`
- `berthacharts_bindings_react.js`
- `berthacharts_bindings_react.d.ts`
- `berthacharts_bindings_react_bg.wasm`
- license files

The package root should point at `react.js` and `react.d.ts`. The exports map
should expose `.` for React and `./wasm` for the raw wasm-pack module.

## Testing

Verification for this milestone:

- `cargo test --workspace --all-targets`
- `scripts/build-npm.sh`
- `npm --cache /private/tmp/bertha-npm-cache pack --dry-run` from
  `crates/bindings-react/pkg`
- A Node import smoke test for `react.js` that uses a stub `react` package and
  verifies the public exports are loadable.

## Non-Goals

- Publishing to npm or crates.io.
- Adding new chart types.
- Replacing wasm-pack.
- Reworking the renderer.
- Introducing a TypeScript compiler or bundler into the package build.
