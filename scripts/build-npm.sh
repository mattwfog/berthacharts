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
cp "${ROOT}/crates/bindings-react/js/react.js" .
cp "${ROOT}/crates/bindings-react/js/react.d.ts" .

# wasm-pack copies the crate's LICENSE files into pkg/ but its generated `files`
# allowlist omits them, so they're absent from the tarball. Add them explicitly.
echo "==> bundling license files"
cp "${ROOT}/LICENSE-MIT" "${ROOT}/LICENSE-APACHE" .

node <<'NODE'
const fs = require("node:fs");

const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
pkg.name = "@berthacharts/react";
pkg.description = "React components and WASM bindings for Bertha Charts.";
pkg.main = "react.js";
pkg.module = "react.js";
pkg.types = "react.d.ts";
pkg.files = [
  "react.js",
  "react.d.ts",
  "berthacharts_bindings_react_bg.wasm",
  "berthacharts_bindings_react.js",
  "berthacharts_bindings_react.d.ts",
  "LICENSE-APACHE",
  "LICENSE-MIT",
];
pkg.exports = {
  ".": {
    types: "./react.d.ts",
    import: "./react.js",
  },
  "./wasm": {
    types: "./berthacharts_bindings_react.d.ts",
    import: "./berthacharts_bindings_react.js",
  },
};
pkg.peerDependencies = {
  ...(pkg.peerDependencies || {}),
  react: ">=18",
};
pkg.sideEffects = pkg.sideEffects || ["./snippets/*"];

fs.writeFileSync("package.json", `${JSON.stringify(pkg, null, 2)}\n`);
NODE

echo "==> done. package ready at: $(pwd)"
echo "    name:    $(node -p "require('./package.json').name")"
echo "    version: $(node -p "require('./package.json').version")"
echo "    publish: ( cd $(pwd) && npm publish --access public )"
