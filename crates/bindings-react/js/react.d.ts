import type { CSSProperties, ReactElement } from "react";
import type { BerthaChart } from "./berthacharts_bindings_react";

export type ChartType =
  | "bar"
  | "line"
  | "area"
  | "sparkline"
  | "scatter"
  | "heatmap"
  | "sankey";

export interface BarDatum {
  label: string;
  value: number;
}

export interface LineDatum {
  series: string;
  x: number;
  y: number;
  label?: string;
}

export interface AreaDatum {
  series: string;
  x: number;
  y: number;
}

export interface SparklineDatum {
  x: number;
  y: number;
}

export interface ScatterDatum {
  label: string;
  x: number;
  y: number;
  group?: string;
  radius?: number;
}

export interface HeatmapCell {
  row: string;
  column: string;
  value: number;
  baseline?: number;
  labelDetail?: string;
  label_detail?: string;
}

export interface SankeyFlow {
  source: string;
  target: string;
  value: number;
  class?: string;
}

export interface SankeyStage {
  index: number;
  label: string;
}

export interface BarChartOptions {
  target?: number;
  xLabel?: string;
  yLabel?: string;
  x_label?: string;
  y_label?: string;
  yMax?: number;
  y_max?: number;
  yTicks?: number;
  y_ticks?: number;
}

export interface LineChartOptions {
  xLabel?: string;
  yLabel?: string;
  x_label?: string;
  y_label?: string;
  xTicks?: number;
  yTicks?: number;
  x_ticks?: number;
  y_ticks?: number;
  lineWidth?: number;
  line_width?: number;
  showPoints?: boolean;
  show_points?: boolean;
}

export interface AreaChartOptions {
  padding?: number;
  stack?: "overlap" | "stacked" | "normalized";
  overlapFillOpacity?: number;
  overlap_fill_opacity?: number;
  showLine?: boolean;
  show_line?: boolean;
  lineWidth?: number;
  line_width?: number;
  yDomain?: [number, number];
  y_domain?: [number, number];
}

export interface SparklineOptions {
  padding?: number;
  stroke?: [number, number, number, number];
  lineWidth?: number;
  line_width?: number;
  fill?: [number, number, number, number];
  dots?: "none" | "min_max" | "minMax" | "first_last" | "firstLast" | "all";
  dotColor?: [number, number, number, number];
  dot_color?: [number, number, number, number];
  dotRadius?: number;
  dot_radius?: number;
  baseline?: boolean;
  baselineColor?: [number, number, number, number];
  baseline_color?: [number, number, number, number];
  yDomain?: [number, number];
  y_domain?: [number, number];
}

export interface ScatterPlotOptions {
  xLabel?: string;
  yLabel?: string;
  x_label?: string;
  y_label?: string;
  xTicks?: number;
  yTicks?: number;
  x_ticks?: number;
  y_ticks?: number;
}

export interface HeatmapOptions {
  signalThreshold?: number;
  signal_threshold?: number;
  legendTitle?: string;
  legend_title?: string;
  maxVisibleLabels?: number;
  max_visible_labels?: number;
}

export interface SankeyOptions {
  labels?: Record<string, string>;
  order?: Record<string, number>;
  stages?: SankeyStage[];
}

export interface TickGuide {
  position: number;
  label: string;
}

export interface AxisGuide {
  orient: "top" | "right" | "bottom" | "left";
  label?: string;
  ticks: TickGuide[];
}

export interface LabelGuide {
  x: number;
  y: number;
  text: string;
  detail?: string;
  anchor:
    | "center"
    | "top"
    | "bottom"
    | "left"
    | "right"
    | "top-left"
    | "top-right"
    | "bottom-left"
    | "bottom-right";
}

export interface LegendItemGuide {
  label: string;
  color: string;
}

export interface LegendGuide {
  title?: string;
  anchor:
    | "top"
    | "bottom"
    | "top-left"
    | "top-right"
    | "bottom-left"
    | "bottom-right";
  items: LegendItemGuide[];
}

export interface PlotAreaGuide {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Guides {
  axes: AxisGuide[];
  labels: LabelGuide[];
  legend?: LegendGuide;
  plot_area: PlotAreaGuide;
}

export interface ChartSize {
  width: number;
  height: number;
}

export interface CommonChartProps<TData, TOptions> {
  data: TData;
  options?: TOptions;
  width?: number;
  height?: number;
  wasmUrl?: string | URL | Response | BufferSource | WebAssembly.Module;
  className?: string;
  overlayClassName?: string;
  style?: CSSProperties;
  ariaLabel?: string;
}

export interface BerthaChartCanvasProps<TData = unknown, TOptions = unknown>
  extends CommonChartProps<TData, TOptions> {
  type: ChartType;
}

export type BarChartProps = CommonChartProps<BarDatum[], BarChartOptions>;
export type LineChartProps = CommonChartProps<LineDatum[], LineChartOptions>;
export type AreaChartProps = CommonChartProps<AreaDatum[], AreaChartOptions>;
export type SparklineProps = CommonChartProps<
  SparklineDatum[],
  SparklineOptions
>;
export type ScatterPlotProps = CommonChartProps<
  ScatterDatum[],
  ScatterPlotOptions
>;
export type HeatmapProps = CommonChartProps<HeatmapCell[], HeatmapOptions>;
export type SankeyProps = CommonChartProps<SankeyFlow[], SankeyOptions>;

export interface UseBerthaChartArgs<TData = unknown, TOptions = unknown> {
  type: ChartType;
  data: TData;
  options?: TOptions;
  width?: number;
  height?: number;
  wasmUrl?: string | URL | Response | BufferSource | WebAssembly.Module;
}

export interface UseBerthaChartResult {
  canvasRef: { current: HTMLCanvasElement | null };
  guides: Guides | null;
  error: unknown;
  size: ChartSize;
}

export function initBerthaCharts(
  wasmUrl?: string | URL | Response | BufferSource | WebAssembly.Module,
): Promise<unknown>;

export function useBerthaChart<TData = unknown, TOptions = unknown>(
  args: UseBerthaChartArgs<TData, TOptions>,
): UseBerthaChartResult;

export function BerthaChartCanvas<TData = unknown, TOptions = unknown>(
  props: BerthaChartCanvasProps<TData, TOptions>,
): ReactElement;

export function BarChart(props: BarChartProps): ReactElement;
export function LineChart(props: LineChartProps): ReactElement;
export function AreaChart(props: AreaChartProps): ReactElement;
export function Sparkline(props: SparklineProps): ReactElement;
export function ScatterPlot(props: ScatterPlotProps): ReactElement;
export function Heatmap(props: HeatmapProps): ReactElement;
export function Sankey(props: SankeyProps): ReactElement;

export { BerthaChart };
export { default as initWasm } from "./berthacharts_bindings_react";
