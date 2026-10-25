use crate::backtest::BacktestResult;
use serde::Serialize;

// Aggregate metrics only. No market-price series or trades leave the Rust backend.
#[derive(Debug, Serialize)]
pub struct TradePerformance {
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub breakeven_trades: usize,
    pub win_rate_pct: Option<f64>,
    pub realized_net_pnl: f64,
    pub open_net_pnl: f64,
}

pub fn summarize_trades(
    result: &BacktestResult,
    starting_cash: f64,
) -> Result<TradePerformance, String> {
    if !starting_cash.is_finite()
        || !result.ending_equity.is_finite()
        || !result.ending_cash.is_finite()
        || result.trades.iter().any(|t| !t.net_pnl.is_finite())
    {
        return Err("Non-finite performance values".into());
    }

    let winning_trades = result.trades.iter().filter(|t| t.net_pnl > 0.0).count();
    let losing_trades = result.trades.iter().filter(|t| t.net_pnl < 0.0).count();
    let breakeven_trades = result.trades.len() - winning_trades - losing_trades;
    let realized_net_pnl: f64 = result.trades.iter().map(|t| t.net_pnl).sum();

    // Net P&L remaining in an open position, INCLUDING its entry fee and
    // slippage. No hypothetical sale fee is charged before an actual exit.
    // This follows from final equity = starting cash + realized + open P&L.
    let outstanding = result.ending_equity - starting_cash - realized_net_pnl;
    if !outstanding.is_finite() {
        return Err("Invalid open-position P&L".into());
    }
    if result.open_shares == 0 && outstanding.abs() > 0.01 {
        return Err("Closed-position equity does not reconcile".into());
    }
    let open_net_pnl = if result.open_shares == 0 {
        0.0
    } else {
        outstanding
    };
    let win_rate_pct = if result.trades.is_empty() {
        None
    } else {
        Some(100.0 * winning_trades as f64 / result.trades.len() as f64)
    };

    Ok(TradePerformance {
        winning_trades,
        losing_trades,
        breakeven_trades,
        win_rate_pct,
        realized_net_pnl,
        open_net_pnl,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backtest::Trade;

    fn trade(pnl: f64) -> Trade {
        Trade {
            entry_date: "2023-01-03".into(),
            exit_date: "2023-01-04".into(),
            shares: 1,
            entry_price: 10.0,
            exit_price: 10.0 + pnl,
            net_pnl: pnl,
            exit_reason: crate::backtest::ExitReason::TimeLimit,
        }
    }

    fn run(pnls: &[f64], equity: f64, shares: u64) -> BacktestResult {
        BacktestResult {
            ending_cash: equity,
            ending_equity: equity,
            open_shares: shares,
            trades: pnls.iter().map(|&v| trade(v)).collect(),
        }
    }

    #[test]
    fn computes_closed_trade_counts_and_win_rate() {
        let stats = summarize_trades(&run(&[10.0, -4.0, 0.0], 106.0, 0), 100.0).unwrap();
        assert_eq!(stats.winning_trades, 1);
        assert_eq!(stats.losing_trades, 1);
        assert_eq!(stats.breakeven_trades, 1);
        assert!((stats.win_rate_pct.unwrap() - 100.0 / 3.0).abs() < 1e-8);
        assert!((stats.realized_net_pnl - 6.0).abs() < 1e-8);
        assert_eq!(stats.open_net_pnl, 0.0);
    }

    #[test]
    fn separates_open_from_realized_profit() {
        let stats = summarize_trades(&run(&[7.0], 120.0, 3), 100.0).unwrap();
        assert!((stats.realized_net_pnl - 7.0).abs() < 1e-8);
        assert!((stats.open_net_pnl - 13.0).abs() < 1e-8);
    }

    #[test]
    fn no_trades_has_no_win_rate() {
        let stats = summarize_trades(&run(&[], 100.0, 0), 100.0).unwrap();
        assert!(stats.win_rate_pct.is_none());
        assert_eq!(stats.winning_trades, 0);
    }

    #[test]
    fn rejects_unexplained_closed_equity() {
        assert!(summarize_trades(&run(&[4.0], 110.0, 0), 100.0).is_err());
    }
}
