# Release Checklist

Bertha Charts publishes the implemented crates first, then the user-facing
facade. The only crate marked `publish = false` is `berthacharts-bindings-react`,
which ships to npm as `@berthacharts/react` instead of crates.io.

## Preflight

Run these checks from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo build --target wasm32-unknown-unknown -p berthacharts-leptos
scripts/build-npm.sh
(cd crates/bindings-react/pkg && npm --cache /private/tmp/bertha-npm-cache pack --dry-run)
node scripts/check-npm-react.mjs crates/bindings-react/pkg
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
7. `berthacharts-anno`
8. `berthacharts-dist`
9. `berthacharts-finance`
10. `berthacharts-renderer-wgpu`
11. `berthacharts-leptos`
12. `berthacharts`

After each `cargo publish -p <crate-name>`, wait for crates.io indexing before
packaging or publishing crates that depend on it.

## Npm Publish

Build and verify the React package from the repository root:

```sh
scripts/build-npm.sh
(cd crates/bindings-react/pkg && npm --cache /private/tmp/bertha-npm-cache pack --dry-run)
node scripts/check-npm-react.mjs crates/bindings-react/pkg
```

Publish the generated package after verifying npm auth and access to the
`@berthacharts` organization:

```sh
(cd crates/bindings-react/pkg && npm publish --access public)
```

## Public Launch

- Tag the release after all crates are published.
- Verify README install instructions against the published facade crate.
- Verify README install instructions against the published npm package.
- Publish the hosted gallery from `examples/leptos-gallery`.
- Add screenshots or GIFs of the gallery to the GitHub release notes.
