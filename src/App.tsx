import BacktestDemo from "./components/BacktestDemo";
import { useEffect, useState } from "react";
import "./theme.css";
import PriceChart from "./components/PriceChart";
import type { DemoRun } from "./types/backtest";

type ThemeName =
  | "main"
  | "catppuccin"
  | "dracula"
  | "aura"
  | "tokyo-night"
  | "rose-pine";

const themes: ThemeName[] = [
  "main",
  "catppuccin",
  "dracula",
  "aura",
  "tokyo-night",
  "rose-pine",
];

export default function App() {
  const [demo, setDemo] = useState<DemoRun | null>(null);
  const [theme, setTheme] = useState<ThemeName>("main");

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand-block">
          <div className="brand-mark">◢</div>
          <div>
            <div className="brand-name">Mihakk</div>
            <div className="brand-sub" lang="ar" dir="rtl">مِحَكّ</div>
          </div>
        </div>

        <div className="topbar-right">
          <span className="build-pill">v0.1 foundation</span>
        </div>
      </header>

      <section className="hero-card">
        <p className="eyebrow">STOCK STRATEGY LAB</p>
        <h1 className="hero-title">Test the strategy. Trust the evidence.</h1>
        <p className="hero-copy">
          Research historical US stocks, backtest trading ideas,
          compare results, and validate your edge before risking real money.
        </p>
      </section>

      <section className="chart-panel">
        <div className="chart-heading">
          <h2>Historical Price Chart</h2>
          <p>{demo ? "Synthetic Rust backtest candles + trade markers" : "Synthetic sample data - not real market prices"}</p>
        </div>
        <PriceChart theme={theme} demo={demo} />
      </section>
      <BacktestDemo onComplete={setDemo} />
      <section className="panel-grid">
        <article className="panel-card">
          <span className="panel-label">Application</span>
          <strong className="panel-value">Windows desktop ready</strong>
          <p className="panel-copy">Tauri v2 + React + Rust foundation is working.</p>
        </article>

        <article className="panel-card">
          <span className="panel-label">Backtesting Engine</span>
          <strong className="panel-value">Rust prototype available</strong>
          <p className="panel-copy">Verified synthetic backtest available below.</p>
        </article>

        <article className="panel-card">
          <span className="panel-label">Theme System</span>
          <strong className="panel-value">Centralized in theme.css</strong>
          <p className="panel-copy">No hard-coded color system in components.</p>
        </article>
      </section>

      <section className="theme-card">
        <div className="section-head">
          <div>
            <h2>Theme Preview</h2>
            <p>Choose one of the six default themes for Mihakk.</p>
          </div>
        </div>

        <div className="theme-switcher">
          {themes.map((item) => (
            <button
              key={item}
              type="button"
              className={item === theme ? "theme-btn active" : "theme-btn"}
              onClick={() => setTheme(item)}
            >
              {item}
            </button>
          ))}
        </div>
      </section>

      <footer className="footer-bar">
        <span>Local-first</span>
        <span>Stocks only</span>
        <span>Research & simulation</span>
      </footer>
    </main>
  );
}


