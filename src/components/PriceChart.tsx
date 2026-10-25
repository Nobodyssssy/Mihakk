import { useEffect, useRef } from "react";
import {
  createChart, CandlestickSeries, ColorType,
  type CandlestickData, type Time, type IChartApi
} from "lightweight-charts";

const sample: CandlestickData<Time>[] = [
  { time: "2026-10-01", open: 10, high: 12, low: 9, close: 11 },
  { time: "2026-10-02", open: 11, high: 14, low: 10, close: 13 },
  { time: "2026-10-05", open: 13, high: 13.5, low: 11, close: 12 },
  { time: "2026-10-06", open: 12, high: 15, low: 11.5, close: 14 },
  { time: "2026-10-07", open: 14, high: 16, low: 13, close: 15 },
  { time: "2026-10-08", open: 15, high: 15.5, low: 12, close: 13 },
  { time: "2026-10-09", open: 13, high: 14, low: 11, close: 12 },
  { time: "2026-10-12", open: 12, high: 15, low: 11, close: 14 },
  { time: "2026-10-13", open: 14, high: 17, low: 13, close: 16 },
  { time: "2026-10-14", open: 16, high: 16.5, low: 14, close: 15 },
  { time: "2026-10-15", open: 15, high: 18, low: 14, close: 17 },
  { time: "2026-10-16", open: 17, high: 19, low: 16, close: 18 },
];

export default function PriceChart({ theme }: { theme: string }) {
  const container = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!container.current) return;
    const element = container.current;
    let chart: IChartApi | null = null;

    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey || event.deltaY === 0) return;

      event.preventDefault();

      if (!chart) return;

      const scale = chart.timeScale();
      const range = scale.getVisibleLogicalRange();
      if (!range) return;

      const span = range.to - range.from;
      if (span <= 0) return;

      const x = event.clientX - element.getBoundingClientRect().left;
      const anchor =
        scale.coordinateToLogical(x) ??
        (range.from + range.to) / 2;

      const factor = Math.exp(
        Math.max(-0.4, Math.min(0.4, event.deltaY * 0.002))
      );

      const nextSpan = Math.max(2, Math.min(100000, span * factor));
      const ratio = nextSpan / span;

      scale.setVisibleLogicalRange({
        from: anchor - (anchor - range.from) * ratio,
        to: anchor + (range.to - anchor) * ratio,
      });
    };

    element.addEventListener("wheel", onWheel, {
      passive: false,
      capture: true,
    });

    const frame = requestAnimationFrame(() => {
      const css = getComputedStyle(document.documentElement);
      const color = (key: string) => css.getPropertyValue(key).trim();

      chart = createChart(element, {
        autoSize: true,
        handleScroll: { mouseWheel: false },
        handleScale: { mouseWheel: false },
        layout: {
          background: { type: ColorType.Solid, color: color("--bg-panel-strong") },
          textColor: color("--text-secondary"),
          attributionLogo: true,
        },
        grid: {
          vertLines: { color: color("--border-soft") },
          horzLines: { color: color("--border-soft") },
        },
      });

      const candles = chart.addSeries(CandlestickSeries, {
        upColor: color("--chart-up"),
        downColor: color("--chart-down"),
        wickUpColor: color("--chart-up"),
        wickDownColor: color("--chart-down"),
        borderVisible: false,
      });

      candles.setData(sample);
      chart.timeScale().fitContent();
    });

    return () => {
      element.removeEventListener("wheel", onWheel, true);
      cancelAnimationFrame(frame);
      chart?.remove();
    };
  }, [theme]);

  return <div ref={container} className="chart-canvas" />;
}

