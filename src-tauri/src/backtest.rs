use crate::market_types::{Candle, validate_and_sort};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Trade {
    pub entry_date: String,
    pub exit_date: String,
    pub shares: u64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub net_pnl: f64,
}

#[derive(Debug, Serialize)]
pub struct BacktestResult {
    pub ending_cash: f64,
    pub ending_equity: f64,
    pub open_shares: u64,
    pub trades: Vec<Trade>,
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
    if !starting_cash.is_finite()
        || starting_cash <= 0.0
        || !fee.is_finite()
        || fee < 0.0
        || !slippage_bps.is_finite()
        || !(0.0..10000.0).contains(&slippage_bps)
    {
        return Err("Invalid backtest parameters".into());
    }

    let candles = validate_and_sort(input.to_vec())?;
    let mut cash = starting_cash;
    let mut position: Option<Position> = None;
    let mut trades = Vec::new();

    // Signals are evaluated only from completed earlier bars.
    for i in 2..candles.len() {
        let today = &candles[i];

        if let Some(open) = &position {
            // Exit at the next session's open, never entry day.
            if i > open.entry_index {
                let price = today.open * (1.0 - slippage_bps / 10000.0);
                let proceeds = open.shares as f64 * price - fee;
                let pnl = proceeds - open.total_cost;

                cash += proceeds;

                trades.push(Trade {
                    entry_date: open.entry_date.clone(),
                    exit_date: today.date.clone(),
                    shares: open.shares,
                    entry_price: open.entry_price,
                    exit_price: price,
                    net_pnl: pnl,
                });

                position = None;
            }
            continue;
        }

        let previous = &candles[i - 1];
        let earlier = &candles[i - 2];

        let signal = previous.close > earlier.high && previous.volume > earlier.volume;

        if !signal || cash <= fee {
            continue;
        }

        let buy_price = today.open * (1.0 + slippage_bps / 10000.0);
        let shares = ((cash - fee) / buy_price).floor() as u64;

        if shares == 0 {
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
    }

    let open_shares = position.as_ref().map_or(0, |p| p.shares);
    let last_close = candles.last().unwrap().close;

    // Open positions are valued at last close, not sold artificially.
    let ending_equity = cash + open_shares as f64 * last_close;

    Ok(BacktestResult {
        ending_cash: cash,
        ending_equity,
        open_shares,
        trades,
    })
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
}
