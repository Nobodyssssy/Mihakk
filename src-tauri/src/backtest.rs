use crate::market_types::{Candle, validate_and_sort};
use crate::risk::{DrawdownReport, DrawdownTracker};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitReason {
    StopLoss,
    TakeProfit,
    TimeLimit,
}

#[derive(Debug, Serialize)]
pub struct Trade {
    pub entry_date: String,
    pub exit_date: String,
    pub shares: u64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub net_pnl: f64,
    pub exit_reason: ExitReason,
}

#[derive(Debug, Serialize)]
pub struct BacktestResult {
    pub ending_cash: f64,
    pub ending_equity: f64,
    pub open_shares: u64,
    pub trades: Vec<Trade>,
}

#[derive(Debug, Default, Serialize)]
pub struct ExitCounts {
    pub stop_loss: usize,
    pub take_profit: usize,
    pub time_limit: usize,
}

pub fn summarize_exit_counts(result: &BacktestResult) -> ExitCounts {
    let mut counts = ExitCounts::default();
    for trade in &result.trades {
        match trade.exit_reason {
            ExitReason::StopLoss => counts.stop_loss += 1,
            ExitReason::TakeProfit => counts.take_profit += 1,
            ExitReason::TimeLimit => counts.time_limit += 1,
        }
    }
    counts
}

struct Position {
    entry_index: usize,
    entry_date: String,
    shares: u64,
    entry_price: f64,
    total_cost: f64,
}

pub fn run_volume_breakout(
    input: &[Candle],
    starting_cash: f64,
    fee: f64,
    slippage_bps: f64,
) -> Result<BacktestResult, String> {
    run_volume_breakout_with_hold(input, starting_cash, fee, slippage_bps, 1)
}

pub fn run_volume_breakout_with_hold(
    input: &[Candle],
    starting_cash: f64,
    fee: f64,
    slippage_bps: f64,
    hold_sessions: u32,
) -> Result<BacktestResult, String> {
    Ok(run_volume_breakout_with_risk(input, starting_cash, fee, slippage_bps, hold_sessions)?.0)
}

pub fn run_volume_breakout_with_risk(
    input: &[Candle],
    starting_cash: f64,
    fee: f64,
    slippage_bps: f64,
    hold_sessions: u32,
) -> Result<(BacktestResult, DrawdownReport), String> {
    run_volume_breakout_with_exits(
        input,
        starting_cash,
        fee,
        slippage_bps,
        hold_sessions,
        0.0,
        0.0,
    )
}

// All thresholds are based on the actual entry fill (including buy slippage).
// Stops and targets become active on the entry day after an opening fill.
// At an opening gap, use the actual open rather than an unattainable threshold.
// If both thresholds are touched within one daily bar, assume STOP first.
fn protective_exit_price(
    candle: &Candle,
    entry_price: f64,
    stop_loss_pct: f64,
    take_profit_pct: f64,
    slippage_bps: f64,
) -> Option<(f64, ExitReason)> {
    let stop = entry_price * (1.0 - stop_loss_pct / 100.0);
    let target = entry_price * (1.0 + take_profit_pct / 100.0);
    let raw = if stop_loss_pct > 0.0 && candle.open <= stop {
        Some((candle.open, ExitReason::StopLoss))
    } else if take_profit_pct > 0.0 && candle.open >= target {
        Some((candle.open, ExitReason::TakeProfit))
    } else if stop_loss_pct > 0.0 && candle.low <= stop {
        Some((stop, ExitReason::StopLoss))
    } else if take_profit_pct > 0.0 && candle.high >= target {
        Some((target, ExitReason::TakeProfit))
    } else {
        None
    };
    raw.map(|(price, reason)| (price * (1.0 - slippage_bps / 10000.0), reason))
}

// Zero means disabled. Explicitly conservative rules for daily OHLC candles.
pub fn run_volume_breakout_with_exits(
    input: &[Candle],
    starting_cash: f64,
    fee: f64,
    slippage_bps: f64,
    hold_sessions: u32,
    stop_loss_pct: f64,
    take_profit_pct: f64,
) -> Result<(BacktestResult, DrawdownReport), String> {
    if !starting_cash.is_finite()
        || starting_cash <= 0.0
        || !fee.is_finite()
        || fee < 0.0
        || !slippage_bps.is_finite()
        || !(0.0..10000.0).contains(&slippage_bps)
        || !(1..=60).contains(&hold_sessions)
        || !stop_loss_pct.is_finite()
        || !(0.0..100.0).contains(&stop_loss_pct)
        || !take_profit_pct.is_finite()
        || !(0.0..=1000.0).contains(&take_profit_pct)
    {
        return Err("Invalid backtest parameters".into());
    }

    let candles = validate_and_sort(input.to_vec())?;
    let mut drawdown = DrawdownTracker::new(starting_cash, &candles[0].date);
    let mut cash = starting_cash;
    let mut position: Option<Position> = None;
    let mut trades = Vec::new();

    // Signals are evaluated only from completed earlier bars.
    for i in 2..candles.len() {
        let today = &candles[i];

        if let Some(open) = &position {
            // Holding-period exit occurs at the opening auction, before intraday checks.
            let exit = if i - open.entry_index >= hold_sessions as usize {
                Some((
                    today.open * (1.0 - slippage_bps / 10000.0),
                    ExitReason::TimeLimit,
                ))
            } else {
                protective_exit_price(
                    today,
                    open.entry_price,
                    stop_loss_pct,
                    take_profit_pct,
                    slippage_bps,
                )
            };

            if let Some((price, exit_reason)) = exit {
                let closed = position.take().expect("checked open position");
                let proceeds = closed.shares as f64 * price - fee;
                let pnl = proceeds - closed.total_cost;
                cash += proceeds;
                trades.push(Trade {
                    entry_date: closed.entry_date,
                    exit_date: today.date.clone(),
                    shares: closed.shares,
                    entry_price: closed.entry_price,
                    exit_price: price,
                    net_pnl: pnl,
                    exit_reason,
                });
            }
            let open_value = position
                .as_ref()
                .map_or(0.0, |p| p.shares as f64 * today.close);
            drawdown.observe(&today.date, cash + open_value)?;
            continue;
        }

        let previous = &candles[i - 1];
        let earlier = &candles[i - 2];

        let signal = previous.close > earlier.high && previous.volume > earlier.volume;

        if !signal || cash <= fee {
            drawdown.observe(&today.date, cash)?;
            continue;
        }

        let buy_price = today.open * (1.0 + slippage_bps / 10000.0);
        let shares = ((cash - fee) / buy_price).floor() as u64;

        if shares == 0 {
            drawdown.observe(&today.date, cash)?;
            continue;
        }

        let total_cost = shares as f64 * buy_price + fee;
        cash -= total_cost;

        position = Some(Position {
            entry_index: i,
            entry_date: today.date.clone(),
            shares,
            entry_price: buy_price,
            total_cost,
        });

        // A bracket placed with the opening buy can execute later the same day.
        if let Some((price, exit_reason)) = protective_exit_price(
            today,
            buy_price,
            stop_loss_pct,
            take_profit_pct,
            slippage_bps,
        ) {
            let closed = position.take().expect("just opened position");
            let proceeds = closed.shares as f64 * price - fee;
            cash += proceeds;
            trades.push(Trade {
                entry_date: closed.entry_date,
                exit_date: today.date.clone(),
                shares: closed.shares,
                entry_price: closed.entry_price,
                exit_price: price,
                net_pnl: proceeds - closed.total_cost,
                exit_reason,
            });
        }
        let open_value = position
            .as_ref()
            .map_or(0.0, |p| p.shares as f64 * today.close);
        drawdown.observe(&today.date, cash + open_value)?;
    }

    let open_shares = position.as_ref().map_or(0, |p| p.shares);
    let last_close = candles.last().unwrap().close;

    // Open positions are valued at last close, not sold artificially.
    let ending_equity = cash + open_shares as f64 * last_close;

    Ok((
        BacktestResult {
            ending_cash: cash,
            ending_equity,
            open_shares,
            trades,
        },
        drawdown.finish(),
    ))
}

#[derive(Serialize)]
pub struct DemoRun {
    pub candles: Vec<Candle>,
    pub result: BacktestResult,
}

#[tauri::command]
pub fn run_demo_backtest() -> Result<DemoRun, String> {
    let bar = |date: &str, open: f64, high: f64, low: f64, close: f64, volume: u64| Candle {
        date: date.to_string(),
        open,
        high,
        low,
        close,
        volume,
    };

    let candles = vec![
        bar("2023-01-03", 9.0, 10.0, 8.0, 9.0, 100),
        bar("2023-01-04", 9.0, 12.0, 9.0, 11.0, 200),
        bar("2023-01-05", 12.0, 13.0, 11.0, 12.0, 220),
        bar("2023-01-06", 13.0, 14.0, 12.0, 13.0, 240),
    ];

    let result = run_volume_breakout(&candles, 100.0, 0.0, 0.0)?;
    Ok(DemoRun { candles, result })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daily_equity_includes_open_position_losses() {
        let mut candles = sample();
        candles[2].low = 9.0;
        candles[2].close = 10.0;
        let (result, risk) = run_volume_breakout_with_risk(&candles, 100.0, 0.0, 0.0, 1).unwrap();
        assert!((result.ending_equity - 108.0).abs() < 1e-8);
        assert!((risk.max_drawdown_pct - 16.0).abs() < 1e-8);
        assert_eq!(risk.trough_date, "2023-01-05");
    }

    fn bar(date: &str, open: f64, high: f64, low: f64, close: f64, volume: u64) -> Candle {
        Candle {
            date: date.into(),
            open,
            high,
            low,
            close,
            volume,
        }
    }

    fn sample() -> Vec<Candle> {
        vec![
            bar("2023-01-03", 9.0, 10.0, 8.0, 9.0, 100),
            bar("2023-01-04", 9.0, 12.0, 9.0, 11.0, 200),
            bar("2023-01-05", 12.0, 13.0, 11.0, 12.0, 220),
            bar("2023-01-06", 13.0, 14.0, 12.0, 13.0, 240),
        ]
    }

    #[test]
    fn holds_two_sessions_before_exit() {
        let mut candles = sample();
        candles.push(bar("2023-01-09", 14.0, 15.0, 13.0, 14.0, 260));
        let run = run_volume_breakout_with_hold(&candles, 100.0, 0.0, 0.0, 2).unwrap();
        assert_eq!(run.trades.len(), 1);
        assert_eq!(run.trades[0].entry_date, "2023-01-05");
        assert_eq!(run.trades[0].exit_date, "2023-01-09");
        assert_eq!(run.trades[0].shares, 8);
        assert!((run.ending_equity - 116.0).abs() < 0.000001);
    }

    #[test]
    fn rejects_invalid_holding_periods() {
        assert!(run_volume_breakout_with_hold(&sample(), 100.0, 0.0, 0.0, 0).is_err());
        assert!(run_volume_breakout_with_hold(&sample(), 100.0, 0.0, 0.0, 61).is_err());
    }
    #[test]
    fn buys_after_signal_and_exits_next_session() {
        let result = run_volume_breakout(&sample(), 100.0, 0.0, 0.0).unwrap();

        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].entry_date, "2023-01-05");
        assert_eq!(result.trades[0].exit_date, "2023-01-06");
        assert_eq!(result.trades[0].shares, 8);
        assert!((result.ending_equity - 108.0).abs() < 0.000001);
    }

    #[test]
    fn transaction_fees_reduce_profit() {
        let result = run_volume_breakout(&sample(), 100.0, 1.0, 0.0).unwrap();

        assert!((result.ending_cash - 106.0).abs() < 0.000001);
        assert!((result.trades[0].net_pnl - 6.0).abs() < 0.000001);
    }

    #[test]
    fn never_enters_before_signal_is_complete() {
        let result = run_volume_breakout(&sample()[..2], 100.0, 0.0, 0.0).unwrap();

        assert!(result.trades.is_empty());
        assert_eq!(result.open_shares, 0);
    }

    #[test]
    fn open_position_is_not_falsely_closed() {
        let result = run_volume_breakout(&sample()[..3], 100.0, 0.0, 0.0).unwrap();

        assert!(result.trades.is_empty());
        assert_eq!(result.open_shares, 8);
        assert!((result.ending_equity - 100.0).abs() < 0.000001);
    }

    #[test]
    fn invalid_costs_are_rejected() {
        assert!(run_volume_breakout(&sample(), 100.0, -1.0, 0.0).is_err());
        assert!(run_volume_breakout(&sample(), 100.0, 0.0, 10000.0).is_err());
    }
    #[test]
    fn counts_stop_loss_exits_without_reclassifying_price_fills() {
        let (result, _) =
            run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 5, 5.0, 0.0).unwrap();
        let counts = summarize_exit_counts(&result);
        assert_eq!(counts.stop_loss, 1);
        assert_eq!(counts.take_profit, 0);
        assert_eq!(counts.time_limit, 0);
        assert_eq!(result.trades[0].exit_reason, ExitReason::StopLoss);
    }

    #[test]
    fn counts_target_and_holding_period_exits_separately() {
        let (target, _) =
            run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 5, 0.0, 10.0).unwrap();
        let t = summarize_exit_counts(&target);
        assert_eq!((t.stop_loss, t.take_profit, t.time_limit), (0, 1, 0));

        let (timed, _) =
            run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 1, 0.0, 0.0).unwrap();
        let t = summarize_exit_counts(&timed);
        assert_eq!((t.stop_loss, t.take_profit, t.time_limit), (0, 0, 1));
    }

    #[test]
    fn exit_counts_reconcile_with_closed_trades_only() {
        let (open, _) =
            run_volume_breakout_with_exits(&sample()[..3], 100.0, 0.0, 0.0, 5, 0.0, 0.0).unwrap();
        assert_eq!(open.open_shares, 8);
        let counts = summarize_exit_counts(&open);
        assert_eq!(
            counts.stop_loss + counts.take_profit + counts.time_limit,
            open.trades.len()
        );
        assert_eq!(counts.stop_loss + counts.take_profit + counts.time_limit, 0);

        let (closed, _) =
            run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 1, 5.0, 0.0).unwrap();
        let counts = summarize_exit_counts(&closed);
        assert_eq!(
            counts.stop_loss + counts.take_profit + counts.time_limit,
            closed.trades.len()
        );
    }

    #[test]
    fn disabled_exits_preserve_existing_behavior() {
        let before = run_volume_breakout_with_risk(&sample(), 100.0, 0.0, 0.0, 1).unwrap();
        let after =
            run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 1, 0.0, 0.0).unwrap();
        assert!((before.0.ending_equity - after.0.ending_equity).abs() < 1e-8);
        assert_eq!(before.0.trades.len(), after.0.trades.len());
    }

    #[test]
    fn stop_can_trigger_on_entry_day() {
        let (result, _) =
            run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 5, 5.0, 0.0).unwrap();
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].entry_date, "2023-01-05");
        assert_eq!(result.trades[0].exit_date, "2023-01-05");
        assert!((result.trades[0].exit_price - 11.4).abs() < 1e-8);
        assert!((result.ending_equity - 95.2).abs() < 1e-8);
    }

    #[test]
    fn downward_gap_fills_at_open_not_at_stop() {
        let mut c = sample();
        c[2].low = 11.5; // No stop on entry day.
        c[3] = bar("2023-01-06", 10.0, 12.0, 9.0, 11.0, 240);
        let (result, _) = run_volume_breakout_with_exits(&c, 100.0, 0.0, 0.0, 5, 5.0, 0.0).unwrap();
        assert_eq!(result.trades.len(), 1);
        assert!((result.trades[0].exit_price - 10.0).abs() < 1e-8);
        assert!((result.ending_equity - 84.0).abs() < 1e-8);
    }

    #[test]
    fn target_fills_at_threshold_on_intraday_touch() {
        let (result, _) =
            run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 5, 0.0, 10.0).unwrap();
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].exit_date, "2023-01-06");
        assert!((result.trades[0].exit_price - 13.2).abs() < 1e-8);
        assert!((result.ending_equity - 109.6).abs() < 1e-8);
    }

    #[test]
    fn upward_gap_fills_at_open_not_target() {
        let mut c = sample();
        c[3] = bar("2023-01-06", 15.0, 16.0, 14.0, 15.0, 240);
        let (result, _) =
            run_volume_breakout_with_exits(&c, 100.0, 0.0, 0.0, 5, 0.0, 10.0).unwrap();
        assert!((result.trades[0].exit_price - 15.0).abs() < 1e-8);
        assert!((result.ending_equity - 124.0).abs() < 1e-8);
    }

    #[test]
    fn simultaneous_intraday_touch_assumes_stop_first() {
        let mut c = sample();
        c[2] = bar("2023-01-05", 12.0, 14.0, 10.0, 12.0, 220);
        let (result, _) =
            run_volume_breakout_with_exits(&c, 100.0, 0.0, 0.0, 5, 10.0, 10.0).unwrap();
        assert_eq!(result.trades.len(), 1);
        assert!((result.trades[0].exit_price - 10.8).abs() < 1e-8);
        assert!((result.ending_equity - 90.4).abs() < 1e-8);
    }

    #[test]
    fn time_exit_at_open_precedes_intraday_stop() {
        let mut c = sample();
        c[2].low = 11.5;
        c[3] = bar("2023-01-06", 13.0, 14.0, 10.0, 13.0, 240);
        let (result, _) = run_volume_breakout_with_exits(&c, 100.0, 0.0, 0.0, 1, 5.0, 0.0).unwrap();
        assert!((result.trades[0].exit_price - 13.0).abs() < 1e-8);
        assert!((result.ending_equity - 108.0).abs() < 1e-8);
    }

    #[test]
    fn rejects_invalid_exit_percentages() {
        for stop in [-1.0, 100.0, f64::NAN] {
            assert!(
                run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 5, stop, 0.0).is_err()
            );
        }
        for target in [-1.0, 1000.01, f64::NAN] {
            assert!(
                run_volume_breakout_with_exits(&sample(), 100.0, 0.0, 0.0, 5, 0.0, target).is_err()
            );
        }
    }
}
