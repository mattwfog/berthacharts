# Graph Improvements Parallel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` or `superpowers:executing-plans` to
> implement only the assigned lane. Do not edit files outside the lane ownership
> listed below.

**Goal:** Improve graph semantics, layout polish, and gallery interactions while
preserving the existing chart/scene/guide architecture.

**Architecture:** Chart crates emit richer `Guide`, `TooltipGuide`,
`LabelGuide`, and snap target metadata. The gallery renders those semantics into
browser UI and normalizes browser pointer coordinates. The WGPU renderer remains
unchanged.

**Tech Stack:** Rust workspace, Leptos gallery, DOM/SVG guide overlay, Cargo
unit tests.

---

## Ownership

- Core chart worker owns only `crates/charts/src/*`.
- Network graph worker owns only `crates/network/src/*`.
- Gallery interaction worker owns only `examples/leptos-gallery/src/*` and
  `examples/leptos-gallery/style.css`.
- Coordinator owns docs, `.gitignore`, integration review, formatting, tests,
  commit, merge, and push.

### Task 1: Core Chart Semantic Polish

**Files:**
- Modify: `crates/charts/src/area.rs`
- Modify: `crates/charts/src/sparkline.rs`
- Optionally modify: `crates/charts/src/lib.rs`
- Optionally create: `crates/charts/src/theme.rs`

- [ ] Add a small shared palette helper if it reduces duplication without
  changing public chart behavior.
- [ ] Extend area chart scene output with a legend and tooltip guide using the
  existing computed dataset columns.
- [ ] Add authored snap targets to area charts for points along the top edge of
  each band.
- [ ] Extend sparkline scene output with endpoint, minimum, and maximum snap
  targets and compact labels if the data density allows them.
- [ ] Add or update unit tests that assert guide counts, tooltip fields, and
  snap target counts for area and sparkline specs.
- [ ] Run `cargo test -p berthacharts-charts`.

### Task 2: Network and Flow Graph Polish

**Files:**
- Modify: `crates/network/src/sankey.rs`
- Modify: `crates/network/src/force.rs`
- Modify: `crates/network/src/chord.rs`
- Modify: `crates/network/src/tree.rs`
- Optionally modify: `crates/network/src/sunburst.rs`

- [ ] Update Sankey ribbon offset ordering so source offsets are ordered by
  target node center and target offsets are ordered by source node center.
- [ ] Preserve link row identity and tooltip lookup after Sankey ordering.
- [ ] Add or improve guide metadata for force, chord, and tree charts where the
  layout already computes node/link coordinates.
- [ ] Adjust tree label anchors by orientation so labels do not sit on top of
  nodes in horizontal layouts.
- [ ] Add focused tests for Sankey ordering and at least one new guide/snap
  behavior in force, chord, or tree.
- [ ] Run `cargo test -p berthacharts-network`.

### Task 3: Gallery Interaction Polish

**Files:**
- Modify: `examples/leptos-gallery/src/chart_canvas.rs`
- Modify: `examples/leptos-gallery/src/guide_overlay.rs`
- Modify: `examples/leptos-gallery/src/annotation_layer.rs`
- Modify: `examples/leptos-gallery/src/chart_chrome.rs`
- Modify: `examples/leptos-gallery/style.css`

- [ ] Replace fixed tooltip placement dimensions with a reusable placement
  helper that clamps to chart-local bounds.
- [ ] Improve label tooltip markup/classes so long rows wrap inside the chart
  instead of overflowing.
- [ ] Convert annotation pointer offsets into logical chart coordinates using
  rendered element size before clamping and snapping.
- [ ] Make sketch controls compact and stable at narrow widths without changing
  the underlying annotation state model.
- [ ] Add focused unit-testable helpers where possible and keep wasm-only code
  guarded.
- [ ] Run the strongest available gallery check, preferring `trunk build` from
  `examples/leptos-gallery` when local tooling supports it.

### Task 4: Integration and Verification

**Files:**
- Modify only as needed after reviewing worker diffs.

- [ ] Review all worker diffs for ownership violations and API conflicts.
- [ ] Run `cargo fmt`.
- [ ] Run `cargo test -p berthacharts-charts`.
- [ ] Run `cargo test -p berthacharts-network`.
- [ ] Run `cargo test --workspace --all-targets`.
- [ ] Run a gallery build or browser smoke if available.
- [ ] Commit the completed milestone on `graph-improvements-parallel`.
- [ ] Merge to `main` and push only after verification.
