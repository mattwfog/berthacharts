# Rust Kernel and React Adoption Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a usable React layer and release-quality package/docs checks while preserving the Rust kernel contract.

**Architecture:** Keep wasm-pack as the WASM build source, add hand-authored React JavaScript and TypeScript declarations as package source, and copy them into the generated npm package during `scripts/build-npm.sh`. Rust remains the layout/rendering engine; React owns lifecycle, resize, props, and DOM guides.

**Tech Stack:** Rust workspace, wasm-pack, npm package metadata, React ESM, plain JavaScript, TypeScript declarations, GitHub Actions.

---

## File Structure

- Create: `crates/bindings-react/js/react.js`
  - React components, hook, lazy WASM initialization, guide overlay rendering.
- Create: `crates/bindings-react/js/react.d.ts`
  - Public npm TypeScript API for component props, data shapes, and guide data.
- Create: `crates/bindings-react/js/README.md`
  - Source note for maintainers explaining why these files are copied into `pkg`.
- Modify: `scripts/build-npm.sh`
  - Copy authored JS/DTS files, stamp idempotent `package.json` metadata, add exports map.
- Modify: `.github/workflows/ci.yml`
  - Add npm package dry-run and import smoke check.
- Modify: `README.md`
  - Update version/status, Rust quick start, npm quick start, and architecture notes.
- Modify: `RELEASE.md`
  - Add npm preflight and publish steps.

### Task 1: Add React Package Source

**Files:**
- Create: `crates/bindings-react/js/react.js`
- Create: `crates/bindings-react/js/react.d.ts`
- Create: `crates/bindings-react/js/README.md`

- [ ] **Step 1: Write `react.js`**

Create `crates/bindings-react/js/react.js` with these exports:

```js
import React, { useEffect, useMemo, useRef, useState } from "react";
import initWasm, { BerthaChart } from "./berthacharts_bindings_react.js";

let initPromise;

export function initBerthaCharts(wasmUrl) {
  if (!initPromise) {
    initPromise = initWasm(wasmUrl);
  }
  return initPromise;
}

export function useBerthaChart({ type, data, options, width, height, wasmUrl }) {
  const canvasRef = useRef(null);
  const chartRef = useRef(null);
  const [guides, setGuides] = useState(null);
  const [error, setError] = useState(null);

  const payload = useMemo(() => JSON.stringify(toPayload(type, data, options)), [type, data, options]);

  useEffect(() => {
    let cancelled = false;
    async function render() {
      const canvas = canvasRef.current;
      if (!canvas) return;
      try {
        await initBerthaCharts(wasmUrl);
        const logicalWidth = Math.max(1, Math.round(width || canvas.clientWidth || 640));
        const logicalHeight = Math.max(1, Math.round(height || canvas.clientHeight || 360));
        if (!chartRef.current) {
          chartRef.current = await BerthaChart.create(canvas, logicalWidth, logicalHeight);
        } else {
          chartRef.current.resize(logicalWidth, logicalHeight);
        }
        chartRef.current[type](payload);
        if (!cancelled) {
          const rawGuides = chartRef.current.guides();
          setGuides(rawGuides ? JSON.parse(rawGuides) : null);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err);
        }
      }
    }
    render();
    return () => {
      cancelled = true;
    };
  }, [type, payload, width, height, wasmUrl]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || width || height || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      const chart = chartRef.current;
      if (!chart) return;
      const logicalWidth = Math.max(1, Math.round(canvas.clientWidth || 640));
      const logicalHeight = Math.max(1, Math.round(canvas.clientHeight || 360));
      chart.resize(logicalWidth, logicalHeight);
      chart[type](payload);
      const rawGuides = chart.guides();
      setGuides(rawGuides ? JSON.parse(rawGuides) : null);
    });
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [type, payload, width, height]);

  useEffect(() => {
    return () => {
      if (chartRef.current) {
        chartRef.current.destroy();
        chartRef.current = null;
      }
    };
  }, []);

  return { canvasRef, guides, error };
}

export function BerthaChartCanvas(props) {
  const { className, style, overlayClassName, width, height, ...chartProps } = props;
  const { canvasRef, guides, error } = useBerthaChart({ ...chartProps, width, height });
  const frameStyle = {
    position: "relative",
    width: width ? `${width}px` : "100%",
    height: height ? `${height}px` : "100%",
    minHeight: height ? undefined : 240,
    ...style,
  };
  return React.createElement(
    "div",
    { className, style: frameStyle, "data-berthacharts": chartProps.type },
    React.createElement("canvas", {
      ref: canvasRef,
      style: { display: "block", width: "100%", height: "100%" },
      "aria-label": props.ariaLabel || `${chartProps.type} chart`,
    }),
    React.createElement(GuidesOverlay, { guides, className: overlayClassName }),
    error ? React.createElement("div", { role: "alert", style: errorStyle }, String(error.message || error)) : null
  );
}
```

Then add helper functions and component wrappers in the same file:

```js
export function BarChart(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "bar" });
}

export function LineChart(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "line" });
}

export function ScatterPlot(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "scatter" });
}

export function Heatmap(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "heatmap" });
}

export function Sankey(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "sankey" });
}

function toPayload(type, data, options = {}) {
  if (type === "heatmap") return { cells: data, ...options };
  if (type === "sankey") {
    const { labels, order, stages, ...rest } = options;
    return { flows: data, labels, order, stages, ...rest };
  }
  return { data, ...options };
}

function GuidesOverlay({ guides, className }) {
  if (!guides) return null;
  const plot = guides.plot_area || { x: 0, y: 0, w: 0, h: 0 };
  const width = Math.max(1, plot.x + plot.w + 64);
  const height = Math.max(1, plot.y + plot.h + 48);
  return React.createElement(
    "svg",
    { className, viewBox: `0 0 ${width} ${height}`, style: overlayStyle, "aria-hidden": "true" },
    guides.axes?.flatMap((axis, index) => renderAxis(axis, index, plot)) || null,
    guides.labels?.map((label, index) =>
      React.createElement("text", {
        key: `label-${index}`,
        x: label.x,
        y: label.y,
        textAnchor: textAnchor(label.anchor),
        dominantBaseline: dominantBaseline(label.anchor),
        fill: "currentColor",
        fontSize: 12,
      }, label.text)
    ) || null,
    guides.legend ? renderLegend(guides.legend, plot) : null
  );
}

function renderAxis(axis, index, plot) {
  const horizontal = axis.orient === "bottom" || axis.orient === "top";
  const baseX = axis.orient === "right" ? plot.x + plot.w : plot.x;
  const baseY = axis.orient === "top" ? plot.y : plot.y + plot.h;
  const line = horizontal
    ? React.createElement("line", { key: `axis-${index}`, x1: plot.x, x2: plot.x + plot.w, y1: baseY, y2: baseY, stroke: "currentColor", opacity: 0.45 })
    : React.createElement("line", { key: `axis-${index}`, x1: baseX, x2: baseX, y1: plot.y, y2: plot.y + plot.h, stroke: "currentColor", opacity: 0.45 });
  const ticks = axis.ticks.map((tick, tickIndex) => {
    const x = horizontal ? tick.position : baseX - 6;
    const y = horizontal ? baseY + 18 : tick.position + 4;
    return React.createElement("text", {
      key: `axis-${index}-tick-${tickIndex}`,
      x,
      y,
      textAnchor: horizontal ? "middle" : "end",
      fill: "currentColor",
      fontSize: 11,
      opacity: 0.78,
    }, tick.label);
  });
  return [line, ...ticks];
}

function renderLegend(legend, plot) {
  const x = plot.x + plot.w - 120;
  const y = plot.y + 8;
  return React.createElement("g", { key: "legend", transform: `translate(${x} ${y})` },
    legend.title ? React.createElement("text", { x: 0, y: 0, fill: "currentColor", fontSize: 12, fontWeight: 600 }, legend.title) : null,
    legend.items.map((item, index) =>
      React.createElement("g", { key: `${item.label}-${index}`, transform: `translate(0 ${18 + index * 18})` },
        React.createElement("rect", { x: 0, y: -9, width: 10, height: 10, fill: item.color }),
        React.createElement("text", { x: 16, y: 0, fill: "currentColor", fontSize: 11 }, item.label)
      )
    )
  );
}

function textAnchor(anchor) {
  if (anchor === "left") return "start";
  if (anchor === "right") return "end";
  return "middle";
}

function dominantBaseline(anchor) {
  if (anchor === "top") return "hanging";
  if (anchor === "bottom") return "baseline";
  return "middle";
}

const overlayStyle = {
  position: "absolute",
  inset: 0,
  width: "100%",
  height: "100%",
  pointerEvents: "none",
  overflow: "visible",
};

const errorStyle = {
  position: "absolute",
  inset: 8,
  color: "#b42318",
  font: "12px system-ui, sans-serif",
  pointerEvents: "none",
};

export { BerthaChart } from "./berthacharts_bindings_react.js";
export { default as initWasm } from "./berthacharts_bindings_react.js";
```

- [ ] **Step 2: Write `react.d.ts`**

Create `crates/bindings-react/js/react.d.ts` with public types for chart data,
guide overlay data, component props, and the exported hook/components.

- [ ] **Step 3: Write source note**

Create `crates/bindings-react/js/README.md` explaining that these files are
authored source copied into wasm-pack output by `scripts/build-npm.sh`.

### Task 2: Make Npm Build Idempotent and React-First

**Files:**
- Modify: `scripts/build-npm.sh`

- [ ] **Step 1: Update package stamping**

After wasm-pack runs, copy `js/react.js` and `js/react.d.ts` into `pkg/`.
Use a Node one-liner to set package metadata in one deterministic write:

```js
pkg.name = "@berthacharts/react";
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
  "LICENSE-MIT"
];
pkg.exports = {
  ".": { "types": "./react.d.ts", "import": "./react.js" },
  "./wasm": {
    "types": "./berthacharts_bindings_react.d.ts",
    "import": "./berthacharts_bindings_react.js"
  }
};
pkg.peerDependencies = { react: ">=18" };
```

- [ ] **Step 2: Run package build**

Run: `scripts/build-npm.sh`

Expected: `crates/bindings-react/pkg/package.json` has no duplicated license
entries and points the package root to `react.js`.

- [ ] **Step 3: Run npm dry-run**

Run: `npm --cache /private/tmp/bertha-npm-cache pack --dry-run`

Expected: the tarball contains `react.js`, `react.d.ts`, WASM files, and
licenses.

### Task 3: Add CI Package Checks

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add wasm-pack npm job**

Install wasm-pack, run `scripts/build-npm.sh`, run `npm pack --dry-run`, create
a temporary stub `react` package, and import `react.js` to verify exports.

- [ ] **Step 2: Keep existing Rust checks unchanged**

The Rust jobs continue to run fmt, MSRV check, clippy, test, docs, and wasm
builds.

### Task 4: Update User Documentation

**Files:**
- Modify: `README.md`
- Modify: `RELEASE.md`

- [ ] **Step 1: Update README status and quick starts**

Set the version examples to `0.0.2`, add `npm install @berthacharts/react`, and
show a small React example using `<BarChart />`.

- [ ] **Step 2: Update architecture section**

Describe React bindings as public npm bindings and clarify which crates remain
incubating.

- [ ] **Step 3: Update release checklist**

Add `scripts/build-npm.sh`, `npm pack --dry-run`, and npm publish instructions.

### Task 5: Verify the Full Milestone

**Files:**
- No source file changes expected.

- [ ] **Step 1: Run Rust tests**

Run: `cargo test --workspace --all-targets`

Expected: all tests pass.

- [ ] **Step 2: Run package build**

Run: `scripts/build-npm.sh`

Expected: package builds and metadata is deterministic.

- [ ] **Step 3: Run package dry-run**

Run: `npm --cache /private/tmp/bertha-npm-cache pack --dry-run` from
`crates/bindings-react/pkg`

Expected: dry-run succeeds and lists the React wrapper files.

- [ ] **Step 4: Run import smoke test**

Run a Node import using a temporary `node_modules/react` stub and verify that
`BarChart`, `LineChart`, `ScatterPlot`, `Heatmap`, `Sankey`, and
`useBerthaChart` are functions.

Expected: Node exits with status 0.
