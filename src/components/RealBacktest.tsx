import { useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";

type RealBacktestSummary = {
  provider: string;
  symbol: string;
  currency: string;
  adjustment: string;
  first_date: string;
  last_date: string;
  bars: number;
  starting_cash: number;
  fee_per_order: number;
  slippage_bps: number;
  ending_cash: number;
  ending_equity: number;
  open_shares: number;
  completed_trades: number;
};

const usd = (n: number) => new Intl.NumberFormat("en-US", {
  style: "currency", currency: "USD",
}).format(n);

export default function RealBacktest() {
  const [apiKey, setApiKey] = useState("");
  const [symbol, setSymbol] = useState("AAPL");
  const [startDate, setStartDate] = useState("2023-01-01");
  const [endDate, setEndDate] = useState("2023-03-31");
  const [startingCash, setStartingCash] = useState("1000");
  const [feePerOrder, setFeePerOrder] = useState("1");
  const [slippageBps, setSlippageBps] = useState("10");
  const [result, setResult] = useState<RealBacktestSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function run() {
    if (!isTauri() || !apiKey.trim() || !symbol.trim()) return;
    if (startDate > endDate) {
      setError("Start date must not be after end date.");
      return;
    }
    const cash = Number(startingCash);
    const fee = Number(feePerOrder);
    const slip = Number(slippageBps);
    if (!Number.isFinite(cash) || cash < 1 || cash > 1_000_000_000 ||
        !Number.isFinite(fee) || fee < 0 || fee > 10_000 ||
        !Number.isFinite(slip) || slip < 0 || slip > 1_000) {
      setError("Check starting cash, fees and slippage values.");
      return;
    }    const key = apiKey.trim();
    setApiKey("");
    setLoading(true);
    setError("");
    setResult(null);
    try {
      const data = await invoke<RealBacktestSummary>("run_real_backtest", {
        apiKey: key,
        symbol: symbol.trim().toUpperCase(),
        startDate,
        endDate,
        startingCash: cash,
        feePerOrder: fee,
        slippageBps: slip,
      });
      setResult(data);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="backtest-panel">
      <h2>Historical research - {symbol.toUpperCase()}</h2>
      <p>Twelve Data daily history. Volume breakout, next-session opening fills. Research only; results are preliminary. For now, choose a range of at least 50 trading sessions.</p>
      <form className="real-market-form" onSubmit={(event) => {
        event.preventDefault();
        void run();
      }}>
        <div className="real-market-filters">
          <label htmlFor="research-symbol">US stock symbol
            <input id="research-symbol" className="real-market-key" type="text"
              value={symbol} maxLength={12} autoCapitalize="characters" spellCheck={false}
              onChange={(event) => { setSymbol(event.target.value.toUpperCase()); setResult(null); }}
              placeholder="AAPL" required />
          </label>
          <label htmlFor="research-start">From
            <input id="research-start" className="real-market-key" type="date"
              value={startDate} onChange={(event) => { setStartDate(event.target.value); setResult(null); }} required />
          </label>
          <label htmlFor="research-end">To (inclusive)
            <input id="research-end" className="real-market-key" type="date"
              value={endDate} onChange={(event) => { setEndDate(event.target.value); setResult(null); }} required />
          </label>
        </div>
        <div className="real-market-filters">
          <label htmlFor="research-capital">Starting USD
            <input id="research-capital" className="real-market-key" type="number"
              min="1" max="1000000000" step="0.01" required
              value={startingCash}
              onChange={(event) => { setStartingCash(event.target.value); setResult(null); }} />
          </label>
          <label htmlFor="research-fee">Fee per order (USD)
            <input id="research-fee" className="real-market-key" type="number"
              min="0" max="10000" step="0.01" required
              value={feePerOrder}
              onChange={(event) => { setFeePerOrder(event.target.value); setResult(null); }} />
          </label>
          <label htmlFor="research-slippage">Slippage (bps)
            <input id="research-slippage" className="real-market-key" type="number"
              min="0" max="1000" step="0.1" required
              value={slippageBps}
              onChange={(event) => { setSlippageBps(event.target.value); setResult(null); }} />
          </label>
        </div>        <label htmlFor="twelve-key">Twelve Data API key</label>
        <input
          id="twelve-key" className="real-market-key" type="password"
          autoComplete="off" value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
          placeholder="Paste key (not saved)"
          aria-label="Twelve Data API key"
        />
        <button className="backtest-run" disabled={loading || !apiKey.trim() || !symbol.trim() || !startDate || !endDate || !isTauri()}>
          {loading ? "Calculating..." : "Run historical backtest"}
        </button>
      </form>
      {!isTauri() && <p>Run this from the Mihakk Windows app.</p>}
      {error && <p role="alert">{error}</p>}
      {result && <div className="backtest-results" aria-live="polite">
        <p>{result.symbol} | {result.first_date} to {result.last_date} | {result.bars} sessions | {result.provider}</p>
        <div className="backtest-metrics">
          <div><span>Starting USD</span><strong>{usd(result.starting_cash)}</strong></div>
          <div><span>Ending equity</span><strong>{usd(result.ending_equity)}</strong></div>
          <div><span>Completed trades</span><strong>{result.completed_trades}</strong></div>
          <div><span>Open shares</span><strong>{result.open_shares}</strong></div>
        </div>
        <p>Assumptions: {usd(result.fee_per_order)} per order; {result.slippage_bps} bps slippage; {result.adjustment}-adjusted data. Unclosed positions are marked to the last close.</p>
      </div>}
      <p className="backtest-disclaimer">
        Free Basic data: internal non-display use. This panel shows aggregate
        research results only; the chart above still uses synthetic data.
        Do not use this prototype to decide real-money trades.
      </p>
    </section>
  );
}
