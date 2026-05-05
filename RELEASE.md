# Release Checklist

Bertha Charts publishes the implemented crates first, then the user-facing
facade. Crates marked `publish = false` are incubating crates and are not part
of the initial public release.

## Preflight

Run these checks from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo build --target wasm32-unknown-unknown -p berthacharts-leptos
```

Run package verification for each publishable crate before publishing it:

```sh
cargo package -p <crate-name>
```

## Publish Order

Publish crates in dependency order:

1. `berthacharts-core`
2. `berthacharts-charts`
3. `berthacharts-transforms`
4. `berthacharts-stats`
5. `berthacharts-geo`
6. `berthacharts-network`
7. `berthacharts-renderer-wgpu`
8. `berthacharts-leptos`
9. `berthacharts`

After each `cargo publish -p <crate-name>`, wait for crates.io indexing before
packaging or publishing crates that depend on it.

## Public Launch

- Tag the release after all crates are published.
- Verify README install instructions against the published facade crate.
- Publish the hosted gallery from `examples/leptos-gallery`.
- Add screenshots or GIFs of the gallery to the GitHub release notes.
