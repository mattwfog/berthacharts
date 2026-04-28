# Bertha Charts

A WebGL chart kernel for Rust and the web. Foundational primitives for
sophisticated, analytically-deep data visualization — consumable from React
(via WASM) and Leptos (natively).

**Status:** v0.0.1 — public traits only, no implementations yet.

## Architecture

- **Core kernel** (`berthacharts-core`) — scales, coordinate systems,
  datasets, scene graph, mark/transform traits, lazy memoized transform DAG,
  picker, event system. No rendering, no framework bindings.
- **Transform crates** (`transforms`, `stats`, `dist`, `finance`, `network`,
  `anno`) — opt-in domain logic. Each implements `Transform` / `Mark` from
  core. Seeded thin at v0.1.
- **Renderer** (`renderer-wgpu`) — wgpu backend targeting WebGL2 on the web
  and native elsewhere.
- **Bindings** — `berthacharts-leptos` (Rust-native) and
  `berthacharts-bindings-react` (WASM + TS).

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

## License

MIT OR Apache-2.0
