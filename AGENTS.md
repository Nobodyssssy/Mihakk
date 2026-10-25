# Mihakk — Development Instructions

These instructions apply to the whole repository. Read them before changing code.

## Identity and architecture

- **App:** Mihakk (مِحَكّ). **Tagline:** “Test the strategy. Trust the evidence.”
- **Purpose:** local-first US-stock research, including penny stocks and small caps, charts, historical backtests and virtual trading. **Stocks only; no cryptocurrency.**
- **Primary platform:** Windows 11; support a future Android build without compromising the desktop UX.
- **Stack:** React + TypeScript + Vite, Tauri v2, Rust, SQLite when persistence is needed, and TradingView Lightweight Charts.
- **Python:** optional separate research/validation environment, never required to run the application.
- No live brokerage integration in the initial versions. Keep real-money systems completely separate from simulations.
- Favor free/open-source tools. Add dependencies only when a required feature justifies them.

## Mandatory design and theme rules

1. **`src/theme.css` is the single source of truth for theme palettes and visual design tokens.** Keep color, typography, sizing, spacing, surfaces, borders, radii and shadows defined centrally. Components reference tokens instead of defining colors.
2. **Never hard-code colors** in React/TypeScript, inline JSX styles, chart configuration or other component styles. For canvas/chart library APIs, resolve CSS custom properties with `getComputedStyle`.
3. Preserve **six default theme IDs** exactly: `main`, `catppuccin`, `dracula`, `aura`, `tokyo-night`, `rose-pine`.
4. **Each theme uses its own cohesive, recognizable palette**—not one generic palette with a swapped accent. Main uses Mihakk's charcoal/emerald identity; the other five follow their respective established design palettes.
5. Separate **UI accents** from **financial semantics**. Use `--accent` for UI emphasis; use `--market-up` / `--market-down` for positive/negative market movement and P/L, with `--chart-up` / `--chart-down` as aliases when useful. Never assume the accent is the positive candle color.
6. Apply the selected theme consistently to candles, wicks, volume bars, indicators, trading markers, profits/losses, chart grids, panels and buttons. Add any new semantic token for every theme or document a deliberate shared fallback.
7. Verify readability and contrast in **all six themes** before accepting UI changes. Do not introduce competing legacy stylesheets that override `theme.css`.
8. Design the Windows desktop workspace for keyboard/mouse first. A future Android interface should be touch-specific while sharing domain logic, models and as much appropriate UI code as possible.
9. Preserve chart interactions: **normal wheel scrolls the page; Ctrl + wheel zooms the chart; dragging pans the timeline; hover reveals price/crosshair.** Do not let the chart hijack ordinary page scrolling. Preserve zoom/viewport during theme changes when practical.
10. Treat the logo mockup as a design direction, not an approved production icon asset.

## Research and backtesting correctness

- Rust is the authoritative, deterministic trading/backtesting engine. UI displays outcomes; it does not independently calculate account balances or fills.
- Start with one daily-bar historical strategy, known test cases and reproducible expected trades/results. Define signal timing and next-eligible-bar execution explicitly to prevent look-ahead bias.
- Account for fees, slippage, position sizing, cash constraints and non-fractional shares by default. Plan explicit handling for stock splits, corporate actions, delistings, survivorship bias and incomplete free data.
- Record data origin and adjusted/raw status, ticker identity, date range, timezone, execution assumptions, strategy settings, starting currency/FX assumptions and engine version.
- Label synthetic prices as **synthetic**. Never present mock data as historical market data or incomplete backtests as proof of profitability.
- AI may suggest research hypotheses/rules, but cannot execute arbitrary generated code in the backtesting engine. Validate rule definitions before simulation.
- Keep virtual funds and real-money trading separate by design.

## Development workflow

- Local Windows path: `E:\Project\Mihakk`. GitHub: `Nobodyssssy/Mihakk`, branch `main`.
- The developer is learning programming. Provide **small, sequential, verifiable steps** with precise PowerShell commands, exact paths and expected output. Do not jump ahead with unnecessary features.
- Inspect existing files before editing; make focused, reversible changes and avoid overwriting working code or adding large dependencies.
- Keep chart UI, market-data import/adapters, simulation engine and persistence modular. Do not grow one monolithic component.
- Verify frontend changes with `npm.cmd run build`; test Rust engine logic with `cargo test` in `src-tauri`; verify relevant UI behavior with `npm.cmd run tauri dev`.
- Avoid committing secrets, `.env` files, API keys, transaction spreadsheets or locally generated outputs. Commit only after verification, with descriptive Git messages.
- **Implementation order:** v0.1 Windows foundation + synthetic chart + historical stock import + one tested Rust backtest; v0.2 strategy editor/costs/risk/comparison; v0.3 CHF 100 paper portfolio and export; v0.4 optional AI research; Android and broader functionality later.

**Priority:** accuracy, reproducibility, modularity and a unified theme system before decorative UI, AI agents or advanced infrastructure.
