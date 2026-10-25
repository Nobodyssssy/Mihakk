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
  hold_sessions: number;
  stop_loss_pct: number;
  take_profit_pct: number;
  ending_cash: number;
  ending_equity: number;
  open_shares: number;
  completed_trades: number;
  buy_hold_drawdown: {
    max_drawdown_pct: number;
    peak_date: string;
    trough_date: string;
  };
  drawdown: {
    max_drawdown_pct: number;
    peak_date: string;
    trough_date: string;
  };
  trade_stats: {
    winning_trades: number;
    losing_trades: number;
    breakeven_trades: number;
    win_rate_pct: number | null;
    realized_net_pnl: number;
    open_net_pnl: number;
  };
  strategy_return_pct: number;
  buy_hold_ending_equity: number;
  buy_hold_return_pct: number;
  buy_hold_shares: number;
  outperformance_pct_points: number;
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
  const [holdSessions, setHoldSessions] = useState("1");
  const [stopLossPct, setStopLossPct] = useState("0");
  const [takeProfitPct, setTakeProfitPct] = useState("0");
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
    const hold = Number(holdSessions);
    const stop = Number(stopLossPct);
    const target = Number(takeProfitPct);
    if (!Number.isFinite(stop) || stop < 0 || stop >= 100 ||
        !Number.isFinite(target) || target < 0 || target > 1000) {
      setError("Stop-loss must be 0 to less than 100%; take-profit 0 to 1000%.");
      return;
    }
    if (!Number.isInteger(hold) || hold < 1 || hold > 60) {
      setError("Holding period must be 1 to 60 trading sessions.");
      return;
    }
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
        holdSessions: hold,
        stopLossPct: stop,
        takeProfitPct: target,
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
        </div>        <div className="real-market-filters">
          <label htmlFor="research-hold">Holding period (trading sessions)
            <input id="research-hold" className="real-market-key" type="number"
              min="1" max="60" step="1" required
              value={holdSessions}
              onChange={(event) => { setHoldSessions(event.target.value); setResult(null); }} />
          </label>
          <label htmlFor="research-stop">Stop-loss (%) - 0 disables
            <input id="research-stop" className="real-market-key" type="number"
              min="0" max="99.99" step="0.1" required
              value={stopLossPct}
              onChange={(event) => { setStopLossPct(event.target.value); setResult(null); }} />
          </label>
          <label htmlFor="research-target">Take-profit (%) - 0 disables
            <input id="research-target" className="real-market-key" type="number"
              min="0" max="1000" step="0.1" required
              value={takeProfitPct}
              onChange={(event) => { setTakeProfitPct(event.target.value); setResult(null); }} />
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
        <h3>Strategy vs. buy-and-hold</h3>
        <div className="backtest-metrics">
          <div>
            <span>Strategy return</span>
            <strong className={result.strategy_return_pct >= 0 ? "pnl-up" : "pnl-down"}>
              {result.strategy_return_pct.toFixed(2)}%
            </strong>
          </div>
          <div>
            <span>Buy-and-hold ending equity</span>
            <strong>{usd(result.buy_hold_ending_equity)}</strong>
          </div>
          <div>
            <span>Buy-and-hold return</span>
            <strong className={result.buy_hold_return_pct >= 0 ? "pnl-up" : "pnl-down"}>
              {result.buy_hold_return_pct.toFixed(2)}%
            </strong>
          </div>
          <div>
            <span>Strategy minus benchmark</span>
            <strong className={result.outperformance_pct_points >= 0 ? "pnl-up" : "pnl-down"}>
              {result.outperformance_pct_points.toFixed(2)} percentage points
            </strong>
          </div>
        </div>
        <p>Benchmark: buy whole shares at the first session's open, with the same
          entry fee and slippage; hold to the final close. Remaining shares
          are valued, not sold. Benchmark shares: {result.buy_hold_shares}.</p>        <h3>Trade outcomes and open-position P&amp;L</h3>
        <div className="backtest-metrics">
          <div>
            <span>Winning / losing / flat trades</span>
            <strong>{result.trade_stats.winning_trades} / {result.trade_stats.losing_trades} / {result.trade_stats.breakeven_trades}</strong>
          </div>
          <div>
            <span>Win rate (completed trades)</span>
            <strong>{result.trade_stats.win_rate_pct === null ? "N/A" : `${result.trade_stats.win_rate_pct.toFixed(1)}%`}</strong>
          </div>
          <div>
            <span>Realized net P&amp;L</span>
            <strong className={result.trade_stats.realized_net_pnl >= 0 ? "pnl-up" : "pnl-down"}>
              {usd(result.trade_stats.realized_net_pnl)}
            </strong>
          </div>
          <div>
            <span>Open-position net P&amp;L</span>
            <strong className={result.trade_stats.open_net_pnl >= 0 ? "pnl-up" : "pnl-down"}>
              {usd(result.trade_stats.open_net_pnl)}
            </strong>
          </div>
        </div>
        <p>Open-position P&amp;L includes the entry fee and slippage, but not a hypothetical exit fee. All P&amp;L values are simulated USD.</p>        <h3>Portfolio risk (daily closes)</h3>
        <div className="backtest-metrics">
          <div>
            <span>Maximum drawdown</span>
            <strong className={result.drawdown.max_drawdown_pct > 0 ? "pnl-down" : undefined}>
              {result.drawdown.max_drawdown_pct.toFixed(2)}%
            </strong>
          </div>
          <div>
            <span>Equity peak date</span>
            <strong>{result.drawdown.max_drawdown_pct > 0 ? result.drawdown.peak_date : "N/A"}</strong>
          </div>
          <div>
            <span>Equity trough date</span>
            <strong>{result.drawdown.max_drawdown_pct > 0 ? result.drawdown.trough_date : "N/A"}</strong>
          </div>
        </div>
        <p>Maximum peak-to-trough decline of simulated equity at each session's close.
          Includes positions still open, entry costs, and completed-trade costs.
          Does not measure intraday drawdowns or hypothetical exit fees.</p>        <h3>Risk compared with buy-and-hold</h3>
        <div className="backtest-metrics">
          <div>
            <span>Strategy maximum drawdown</span>
            <strong className="pnl-down">{result.drawdown.max_drawdown_pct.toFixed(2)}%</strong>
          </div>
          <div>
            <span>Buy-and-hold maximum drawdown</span>
            <strong className="pnl-down">{result.buy_hold_drawdown.max_drawdown_pct.toFixed(2)}%</strong>
          </div>
          <div>
            <span>Benchmark minus strategy drawdown</span>
            <strong className={result.buy_hold_drawdown.max_drawdown_pct >= result.drawdown.max_drawdown_pct ? "pnl-up" : "pnl-down"}>
              {(result.buy_hold_drawdown.max_drawdown_pct - result.drawdown.max_drawdown_pct).toFixed(2)} percentage points
            </strong>
          </div>
          <div>
            <span>Benchmark peak / trough</span>
            <strong>{result.buy_hold_drawdown.max_drawdown_pct > 0 ? `${result.buy_hold_drawdown.peak_date} / ${result.buy_hold_drawdown.trough_date}` : "N/A"}</strong>
          </div>
        </div>
        <p>Both drawdowns use daily closing equity, the same starting cash, fee, and slippage.
          Buy-and-hold enters at the first session's open; the strategy may remain in cash.
          Lower drawdown is not, by itself, evidence of a better strategy.</p>        <p>Assumptions: {usd(result.fee_per_order)} per order; {result.slippage_bps} bps slippage; {result.hold_sessions} trading-session hold; stop-loss {result.stop_loss_pct ? `${result.stop_loss_pct}%` : "off"}; take-profit {result.take_profit_pct ? `${result.take_profit_pct}%` : "off"}; {result.adjustment}-adjusted data. Unclosed positions are marked to the last close.</p>
      </div>}
      <p className="backtest-disclaimer">
        Stops and targets are approximated from daily candles. Opening gaps execute
        at the open; if stop and target are both touched intraday, stop wins.
      </p>
      <p className="backtest-disclaimer">
        Free Basic data: internal non-display use. This panel shows aggregate
        research results only; the chart above still uses synthetic data.
        Do not use this prototype to decide real-money trades.
      </p>
    </section>
  );
}
