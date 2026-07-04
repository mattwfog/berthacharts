import React, { useEffect, useMemo, useRef, useState } from "react";
import initWasm, { BerthaChart } from "./berthacharts_bindings_react.js";

let initPromise;

export function initBerthaCharts(wasmUrl) {
  if (!initPromise) {
    initPromise = initWasm(wasmUrl).catch((error) => {
      initPromise = undefined;
      throw error;
    });
  }
  return initPromise;
}

export function useBerthaChart({
  type,
  data,
  options = {},
  width,
  height,
  wasmUrl,
}) {
  const canvasRef = useRef(null);
  const chartRef = useRef(null);
  const [guides, setGuides] = useState(null);
  const [error, setError] = useState(null);
  const [size, setSize] = useState({
    width: Math.max(1, Math.round(width || 640)),
    height: Math.max(1, Math.round(height || 360)),
  });

  const payload = useMemo(
    () => JSON.stringify(toPayload(type, data, options)),
    [type, data, options],
  );

  useEffect(() => {
    let cancelled = false;

    async function render() {
      const canvas = canvasRef.current;
      if (!canvas) return;

      try {
        await initBerthaCharts(wasmUrl);
        if (cancelled) return;

        const measured = measureCanvas(canvas, width, height);
        setSize(measured);

        if (!chartRef.current) {
          chartRef.current = await BerthaChart.create(
            canvas,
            measured.width,
            measured.height,
          );
        } else {
          chartRef.current.resize(measured.width, measured.height);
        }

        renderPayload(chartRef.current, type, payload);
        if (!cancelled) {
          setGuides(readGuides(chartRef.current));
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
    if (!canvas || width || height || typeof ResizeObserver === "undefined") {
      return undefined;
    }

    const observer = new ResizeObserver(() => {
      const chart = chartRef.current;
      if (!chart) return;

      const measured = measureCanvas(canvas, width, height);
      setSize(measured);
      chart.resize(measured.width, measured.height);
      renderPayload(chart, type, payload);
      setGuides(readGuides(chart));
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

  return { canvasRef, guides, error, size };
}

export function BerthaChartCanvas(props) {
  const {
    className,
    style,
    overlayClassName,
    width,
    height,
    ariaLabel,
    ...chartProps
  } = props;
  const { canvasRef, guides, error, size } = useBerthaChart({
    ...chartProps,
    width,
    height,
  });

  const frameStyle = {
    position: "relative",
    width: width ? `${width}px` : "100%",
    height: height ? `${height}px` : "100%",
    minHeight: height ? undefined : 240,
    color: "currentColor",
    ...style,
  };

  return React.createElement(
    "div",
    { className, style: frameStyle, "data-berthacharts": chartProps.type },
    React.createElement("canvas", {
      ref: canvasRef,
      style: { display: "block", width: "100%", height: "100%" },
      "aria-label": ariaLabel || `${chartProps.type} chart`,
    }),
    React.createElement(GuidesOverlay, {
      guides,
      size,
      className: overlayClassName,
    }),
    error
      ? React.createElement(
          "div",
          { role: "alert", style: errorStyle },
          String(error.message || error),
        )
      : null,
  );
}

export function BarChart(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "bar" });
}

export function LineChart(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "line" });
}

export function AreaChart(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "area" });
}

export function Sparkline(props) {
  return React.createElement(BerthaChartCanvas, { ...props, type: "sparkline" });
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

function measureCanvas(canvas, width, height) {
  return {
    width: Math.max(1, Math.round(width || canvas.clientWidth || 640)),
    height: Math.max(1, Math.round(height || canvas.clientHeight || 360)),
  };
}

function renderPayload(chart, type, payload) {
  if (typeof chart[type] !== "function") {
    throw new Error(`Unsupported Bertha Charts type: ${type}`);
  }
  chart[type](payload);
}

function readGuides(chart) {
  const rawGuides = chart.guides();
  return rawGuides ? JSON.parse(rawGuides) : null;
}

function toPayload(type, data, options = {}) {
  const normalizedOptions = normalizeOptions(type, options);
  if (type === "heatmap") {
    return { cells: normalizeHeatmapCells(data), ...normalizedOptions };
  }
  if (type === "sankey") {
    return { flows: data, ...normalizedOptions };
  }
  return { data, ...normalizedOptions };
}

function normalizeOptions(type, options) {
  const aliases = {
    xLabel: "x_label",
    yLabel: "y_label",
    xTicks: "x_ticks",
    yTicks: "y_ticks",
    yMax: "y_max",
    lineWidth: "line_width",
    overlapFillOpacity: "overlap_fill_opacity",
    showLine: "show_line",
    showPoints: "show_points",
    signalThreshold: "signal_threshold",
    legendTitle: "legend_title",
    maxVisibleLabels: "max_visible_labels",
    dotColor: "dot_color",
    dotRadius: "dot_radius",
    baselineColor: "baseline_color",
    yDomain: "y_domain",
  };
  const normalized = { ...options };
  for (const [from, to] of Object.entries(aliases)) {
    if (normalized[from] !== undefined && normalized[to] === undefined) {
      normalized[to] = normalized[from];
    }
    delete normalized[from];
  }
  if (type !== "sankey") {
    delete normalized.labels;
    delete normalized.order;
    delete normalized.stages;
  }
  return normalized;
}

function normalizeHeatmapCells(cells) {
  return (cells || []).map((cell) => {
    if (cell.labelDetail === undefined || cell.label_detail !== undefined) {
      return cell;
    }
    const normalized = { ...cell, label_detail: cell.labelDetail };
    delete normalized.labelDetail;
    return normalized;
  });
}

function GuidesOverlay({ guides, size, className }) {
  if (!guides || !size) return null;
  const plot = guides.plot_area || { x: 0, y: 0, w: 0, h: 0 };
  return React.createElement(
    "svg",
    {
      className,
      viewBox: `0 0 ${size.width} ${size.height}`,
      style: overlayStyle,
      "aria-hidden": "true",
    },
    guides.axes?.flatMap((axis, index) => renderAxis(axis, index, plot)) ||
      null,
    guides.labels?.map((label, index) =>
      React.createElement(
        "text",
        {
          key: `label-${index}`,
          x: label.x,
          y: label.y,
          textAnchor: textAnchor(label.anchor),
          dominantBaseline: dominantBaseline(label.anchor),
          fill: "currentColor",
          fontSize: 12,
        },
        label.text,
      ),
    ) || null,
    guides.legend ? renderLegend(guides.legend, plot) : null,
  );
}

function renderAxis(axis, index, plot) {
  const horizontal = axis.orient === "bottom" || axis.orient === "top";
  const baseX = axis.orient === "right" ? plot.x + plot.w : plot.x;
  const baseY = axis.orient === "top" ? plot.y : plot.y + plot.h;
  const line = horizontal
    ? React.createElement("line", {
        key: `axis-${index}`,
        x1: plot.x,
        x2: plot.x + plot.w,
        y1: baseY,
        y2: baseY,
        stroke: "currentColor",
        opacity: 0.45,
      })
    : React.createElement("line", {
        key: `axis-${index}`,
        x1: baseX,
        x2: baseX,
        y1: plot.y,
        y2: plot.y + plot.h,
        stroke: "currentColor",
        opacity: 0.45,
      });

  const ticks = (axis.ticks || []).map((tick, tickIndex) => {
    const x = horizontal ? tick.position : baseX - 6;
    const y = horizontal ? baseY + 18 : tick.position + 4;
    return React.createElement(
      "text",
      {
        key: `axis-${index}-tick-${tickIndex}`,
        x,
        y,
        textAnchor: horizontal ? "middle" : "end",
        fill: "currentColor",
        fontSize: 11,
        opacity: 0.78,
      },
      tick.label,
    );
  });

  return [line, ...ticks];
}

function renderLegend(legend, plot) {
  const itemCount = (legend.items || []).length;
  const width = 132;
  const height = 20 + itemCount * 18;
  const { x, y } = legendPosition(legend.anchor, plot, width, height);
  return React.createElement(
    "g",
    { key: "legend", transform: `translate(${x} ${y})` },
    legend.title
      ? React.createElement(
          "text",
          { x: 0, y: 0, fill: "currentColor", fontSize: 12, fontWeight: 600 },
          legend.title,
        )
      : null,
    (legend.items || []).map((item, index) =>
      React.createElement(
        "g",
        {
          key: `${item.label}-${index}`,
          transform: `translate(0 ${18 + index * 18})`,
        },
        React.createElement("rect", {
          x: 0,
          y: -9,
          width: 10,
          height: 10,
          fill: item.color,
        }),
        React.createElement(
          "text",
          { x: 16, y: 0, fill: "currentColor", fontSize: 11 },
          item.label,
        ),
      ),
    ),
  );
}

function legendPosition(anchor, plot, width, height) {
  const margin = 8;
  switch (anchor) {
    case "top-left":
      return { x: plot.x + margin, y: plot.y + margin };
    case "top-right":
      return { x: plot.x + plot.w - width - margin, y: plot.y + margin };
    case "bottom-left":
      return { x: plot.x + margin, y: plot.y + plot.h - height - margin };
    case "bottom-right":
      return {
        x: plot.x + plot.w - width - margin,
        y: plot.y + plot.h - height - margin,
      };
    case "top":
      return { x: plot.x + margin, y: plot.y + margin };
    case "bottom":
      return { x: plot.x + margin, y: plot.y + plot.h - height - margin };
    default:
      return { x: plot.x + plot.w - width - margin, y: plot.y + margin };
  }
}

function textAnchor(anchor) {
  if (anchor?.includes("left")) return "end";
  if (anchor?.includes("right")) return "start";
  if (anchor === "left") return "end";
  if (anchor === "right") return "start";
  return "middle";
}

function dominantBaseline(anchor) {
  if (anchor?.includes("top")) return "baseline";
  if (anchor?.includes("bottom")) return "hanging";
  if (anchor === "top") return "baseline";
  if (anchor === "bottom") return "hanging";
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
