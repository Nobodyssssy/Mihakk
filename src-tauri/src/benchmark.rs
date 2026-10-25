use crate::market_types::{Candle, validate_and_sort};

pub struct BuyHoldResult {
    pub ending_equity: f64,
    pub shares: u64,
}

pub fn run_buy_and_hold(
    input: &[Candle],
    starting_cash: f64,
    fee: f64,
    slippage_bps: f64,
) -> Result<BuyHoldResult, String> {
    if !starting_cash.is_finite()
        || starting_cash <= 0.0
        || !fee.is_finite()
        || fee < 0.0
        || !slippage_bps.is_finite()
        || !(0.0..10000.0).contains(&slippage_bps)
    {
        return Err("Invalid benchmark settings".into());
    }

    let candles = validate_and_sort(input.to_vec())?;
    let first = candles.first().unwrap();
    let last = candles.last().unwrap();
    let buy_price = first.open * (1.0 + slippage_bps / 10000.0);

    // If one whole share is unaffordable, keep cash and charge no order fee.
    let shares = if starting_cash > fee {
        ((starting_cash - fee) / buy_price).floor() as u64
    } else {
        0
    };
    let remaining_cash = if shares > 0 {
        starting_cash - shares as f64 * buy_price - fee
    } else {
        starting_cash
    };
    let ending_equity = remaining_cash + shares as f64 * last.close;

    if !ending_equity.is_finite() {
        return Err("Invalid benchmark equity".into());
    }
    Ok(BuyHoldResult {
        ending_equity,
        shares,
    })
}

pub fn run_buy_and_hold_with_risk(
    input: &[Candle],
    starting_cash: f64,
    fee: f64,
    slippage_bps: f64,
) -> Result<(BuyHoldResult, crate::risk::DrawdownReport), String> {
    let baseline = run_buy_and_hold(input, starting_cash, fee, slippage_bps)?;
    let candles = validate_and_sort(input.to_vec())?;
    let entry_price = candles[0].open * (1.0 + slippage_bps / 10000.0);
    let remaining_cash = if baseline.shares > 0 {
        starting_cash - baseline.shares as f64 * entry_price - fee
    } else {
        starting_cash
    };
    let mut tracker = crate::risk::DrawdownTracker::new(starting_cash, &candles[0].date);
    for candle in &candles {
        let equity = remaining_cash + baseline.shares as f64 * candle.close;
        tracker.observe(&candle.date, equity)?;
    }
    Ok((baseline, tracker.finish()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measures_buy_hold_peak_to_trough_with_open_shares() {
        let prices = vec![
            bar("2023-01-03", 10.0, 11.0),
            bar("2023-01-04", 8.0, 8.0),
            bar("2023-01-05", 9.0, 9.0),
        ];
        let (_, risk) = run_buy_and_hold_with_risk(&prices, 100.0, 0.0, 0.0).unwrap();
        assert!((risk.max_drawdown_pct - (30.0 / 110.0 * 100.0)).abs() < 1e-8);
        assert_eq!(risk.peak_date, "2023-01-03");
        assert_eq!(risk.trough_date, "2023-01-04");
    }

    #[test]
    fn buy_hold_risk_preserves_original_benchmark_result() {
        let before = run_buy_and_hold(&sample(), 100.0, 1.0, 100.0).unwrap();
        let (after, risk) = run_buy_and_hold_with_risk(&sample(), 100.0, 1.0, 100.0).unwrap();
        assert_eq!(before.shares, after.shares);
        assert!((before.ending_equity - after.ending_equity).abs() < 1e-8);
        assert_eq!(risk.max_drawdown_pct, 0.0);
    }

    #[test]
    fn buy_hold_no_shares_has_no_drawdown() {
        let (baseline, risk) = run_buy_and_hold_with_risk(&sample(), 5.0, 1.0, 0.0).unwrap();
        assert_eq!(baseline.shares, 0);
        assert_eq!(risk.max_drawdown_pct, 0.0);
    }

    #[test]
    fn first_day_loss_counts_against_starting_capital() {
        let prices = vec![bar("2023-01-03", 10.0, 9.0), bar("2023-01-04", 9.0, 12.0)];
        let (_, risk) = run_buy_and_hold_with_risk(&prices, 100.0, 0.0, 0.0).unwrap();
        assert!((risk.max_drawdown_pct - 10.0).abs() < 1e-8);
    }

    fn bar(date: &str, open: f64, close: f64) -> Candle {
        Candle {
            date: date.into(),
            open,
            high: open.max(close) + 1.0,
            low: open.min(close) - 1.0,
            close,
            volume: 100,
        }
    }

    fn sample() -> Vec<Candle> {
        vec![bar("2023-01-03", 10.0, 11.0), bar("2023-01-04", 11.0, 12.0)]
    }

    #[test]
    fn buys_first_open_and_values_last_close() {
        let result = run_buy_and_hold(&sample(), 100.0, 0.0, 0.0).unwrap();
        assert_eq!(result.shares, 10);
        assert!((result.ending_equity - 120.0).abs() < 1e-8);
    }

    #[test]
    fn entry_fee_and_slippage_reduce_equity() {
        let result = run_buy_and_hold(&sample(), 100.0, 1.0, 100.0).unwrap();
        assert_eq!(result.shares, 9);
        assert!((result.ending_equity - 116.1).abs() < 1e-8);
    }

    #[test]
    fn no_fee_when_a_share_cannot_be_bought() {
        let result = run_buy_and_hold(&sample(), 5.0, 1.0, 0.0).unwrap();
        assert_eq!(result.shares, 0);
        assert!((result.ending_equity - 5.0).abs() < 1e-8);
    }

    #[test]
    fn rejects_invalid_settings() {
        assert!(run_buy_and_hold(&sample(), 0.0, 0.0, 0.0).is_err());
        assert!(run_buy_and_hold(&sample(), 100.0, -1.0, 0.0).is_err());
        assert!(run_buy_and_hold(&sample(), 100.0, 0.0, f64::NAN).is_err());
    }
}
