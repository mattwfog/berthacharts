#!/usr/bin/env bash
#
# Build the public @berthacharts/react npm package from the React / WASM
# bindings crate.
#
# wasm-pack names the generated package after the Rust crate
# (berthacharts-bindings-react); npm wants the scoped public name. This script
# builds the optimized release wasm, then re-stamps the package name (and a few
# npm-facing fields) on the generated pkg/package.json.
#
# Usage (from anywhere):
#   scripts/build-npm.sh            # release build -> crates/bindings-react/pkg
#
# Publish (requires npm auth + the @berthacharts org; run by a maintainer):
#   ( cd crates/bindings-react/pkg && npm publish --access public )
#
set -euo pipefail

PKG_NAME="@berthacharts/react"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> wasm-pack release build (target web)"
wasm-pack build crates/bindings-react --target web --release --out-dir pkg

cd crates/bindings-react/pkg

echo "==> stamping npm package name: ${PKG_NAME}"
npm pkg set name="${PKG_NAME}"

echo "==> done. package ready at: $(pwd)"
echo "    name:    $(npm pkg get name)"
echo "    version: $(npm pkg get version)"
echo "    publish: ( cd $(pwd) && npm publish --access public )"
