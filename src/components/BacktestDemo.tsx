import { useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";

type Trade = {
  entry_date: string;
  exit_date: string;
  shares: number;
  entry_price: number;
  exit_price: number;
  net_pnl: number;
};

type BacktestResult = {
  ending_cash: number;
  ending_equity: number;
  open_shares: number;
  trades: Trade[];
};

const usd = (value: number) =>
  new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
  }).format(value);

export default function BacktestDemo() {
  const [result, setResult] = useState<BacktestResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function runBacktest() {
    setLoading(true);
    setError("");

    try {
      const data = await invoke<BacktestResult>("run_demo_backtest");
      setResult(data);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="backtest-panel">
      <h2>Backtesting Engine</h2>
      <p>
        Synthetic volume-breakout example. Run the verified
        Rust engine and inspect its transactions.
      </p>

      <button
        className="backtest-run"
        onClick={runBacktest}
        disabled={loading || !isTauri()}
      >
        {loading ? "Running..." : "Run Backtest"}
      </button>

      {!isTauri() && (
        <p>Open the Windows app to execute Rust backtests.</p>
      )}

      {error && <p role="alert">{error}</p>}

      {result && (
        <div className="backtest-results" aria-live="polite">
          <div className="backtest-metrics">
            <div>
              <span>Ending equity</span>
              <strong>{usd(result.ending_equity)}</strong>
            </div>
            <div>
              <span>Remaining cash</span>
              <strong>{usd(result.ending_cash)}</strong>
            </div>
            <div>
              <span>Completed trades</span>
              <strong>{result.trades.length}</strong>
            </div>
          </div>

          <div className="backtest-table-wrap">
            <table className="backtest-table">
              <thead>
                <tr>
                  <th>Entry</th>
                  <th>Exit</th>
                  <th>Shares</th>
                  <th>Buy</th>
                  <th>Sell</th>
                  <th>Net P&L</th>
                </tr>
              </thead>
              <tbody>
                {result.trades.map((trade, index) => (
                  <tr key={index}>
                    <td>{trade.entry_date}</td>
                    <td>{trade.exit_date}</td>
                    <td>{trade.shares}</td>
                    <td>{usd(trade.entry_price)}</td>
                    <td>{usd(trade.exit_price)}</td>
                    <td className={
                      trade.net_pnl >= 0 ? "pnl-up" : "pnl-down"
                    }>
                      {usd(trade.net_pnl)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="backtest-disclaimer">
            Synthetic fixture, separate from the sample chart.
            No real stock prices or transactions.
          </p>
        </div>
      )}
    </section>
  );
}
