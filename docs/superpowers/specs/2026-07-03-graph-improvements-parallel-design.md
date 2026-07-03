# Graph Improvements Parallel Design

## Goal

Ship a first graph-quality milestone that makes existing charts feel more
complete without changing the renderer contract or adding new chart families.

## Scope

This pass improves three independent surfaces:

- Core chart specs emit richer semantic guides for overlays and annotations.
- Network/flow specs reduce avoidable layout rough edges and expose more hover
  and label metadata.
- The Leptos gallery handles guides, tooltips, responsive scaling, and drawing
  controls more reliably.

The work is intentionally scoped to the current scene, guide, pick, and snap
target APIs. The renderer continues to own marks and geometry. DOM/SVG overlays
continue to own text, legends, axes, tooltips, and annotation interaction.

## Architecture

Rust chart crates should describe intent through `Guide`, `TooltipGuide`,
`LegendGuide`, `LabelGuide`, and authored snap targets. They should not embed
browser layout behavior. The gallery should render the guide semantics into
stable browser UI, keep tooltip placement viewport-aware, and translate pointer
events into chart-local coordinates before picking or sketching.

The parallel lanes have strict file ownership:

- Core chart lane: `crates/charts/src/*`
- Network lane: `crates/network/src/*`
- Gallery lane: `examples/leptos-gallery/src/*` and
  `examples/leptos-gallery/style.css`

Shared integration and verification are done after the lanes land.

## Milestone Behaviors

Core charts should close the biggest semantic gaps:

- Area charts include legends, tooltip metadata, and useful snap targets across
  overlap, stacked, and normalized modes.
- Sparklines include endpoint, minimum, and maximum snap targets plus compact
  labels where useful.
- Existing chart colors remain compatible while using a common palette helper
  for new guide colors.

Network and flow charts should improve readability without a broad rewrite:

- Sankey ribbon offsets should be ordered by the adjacent node position to
  reduce crossings and preserve source link row identity.
- Force, chord, and tree charts should expose first-class guides similar to
  Sankey and sunburst where their layouts already contain node/link metadata.
- Chord self-links and tree label placement should avoid the most visible
  misleading label/arc behavior.

Gallery interaction should make the current examples feel more deliberate:

- Chart tooltips should clamp against the rendered chart bounds instead of
  relying on fixed dimensions.
- Label tooltips should use the same robust sizing and wrapping conventions as
  chart tooltips.
- Annotation drawing should convert browser event offsets through the same
  responsive chart scale used by picking.
- The draw controls should remain compact and stable at narrow widths.

## Non-Goals

- Replacing WGPU rendering or changing mark tessellation contracts.
- Adding a new public overlay protocol for React in this pass.
- Rebuilding gallery navigation or turning examples into a marketing site.
- Introducing third-party layout engines.

## Verification

Required local verification:

- `cargo test -p berthacharts-charts`
- `cargo test -p berthacharts-network`
- `cargo test --workspace --all-targets`

Gallery verification should include a build or browser smoke where tooling is
available. If local tooling blocks that check, the failure and reason must be
reported.
