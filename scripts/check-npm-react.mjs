import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const packageDir = path.resolve(process.argv[2] ?? "crates/bindings-react/pkg");
const reactStubDir = path.join(packageDir, "node_modules", "react");

async function installReactStub() {
  await mkdir(reactStubDir, { recursive: true });
  await writeFile(
    path.join(reactStubDir, "package.json"),
    JSON.stringify({ name: "react", type: "module", main: "index.js" }, null, 2),
  );
  await writeFile(
    path.join(reactStubDir, "index.js"),
    [
      "export function createElement(type, props, ...children) {",
      "  return { type, props: props || {}, children };",
      "}",
      "export function useEffect() {}",
      "export function useMemo(fn) { return fn(); }",
      "export function useRef(value) { return { current: value }; }",
      "export function useState(value) { return [value, () => {}]; }",
      "export default { createElement, useEffect, useMemo, useRef, useState };",
      "",
    ].join("\n"),
  );
}

async function main() {
  await installReactStub();
  try {
    const moduleUrl = pathToFileURL(path.join(packageDir, "react.js")).href;
    const mod = await import(moduleUrl);
    const expectedFunctions = [
      "BarChart",
      "LineChart",
      "ScatterPlot",
      "Heatmap",
      "Sankey",
      "BerthaChartCanvas",
      "useBerthaChart",
      "initBerthaCharts",
    ];

    for (const name of expectedFunctions) {
      if (typeof mod[name] !== "function") {
        throw new Error(`Expected ${name} to be exported as a function`);
      }
    }

    if (typeof mod.BerthaChart !== "function") {
      throw new Error("Expected raw BerthaChart class to be re-exported");
    }
  } finally {
    await rm(path.join(packageDir, "node_modules"), { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
