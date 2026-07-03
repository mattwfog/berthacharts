# React package source

These files are hand-authored npm package source files. `wasm-pack` generates
the raw WASM module into `crates/bindings-react/pkg/`; `scripts/build-npm.sh`
then copies this directory's files into that generated package and points the
package root at `react.js`.

Keep this directory free of build-tool assumptions. The package should remain
plain ESM plus TypeScript declarations so maintainers can publish it with only
Rust, wasm-pack, and npm available.
