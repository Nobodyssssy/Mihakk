use crate::backtest::run_volume_breakout_with_exits;
use crate::market_data::HistoryDataset;
use crate::market_types::validate_and_sort;
use serde::Serialize;

const STARTING_CASH_USD: f64 = 1000.0;
const FEE_USD: f64 = 1.0;
const SLIPPAGE_BPS: f64 = 10.0;

// Aggregate-only result: do not send licensed raw candles to the UI.
#[derive(Serialize)]
pub struct RealBacktestSummary {
    pub provider: String,
    pub symbol: String,
    pub currency: String,
    pub adjustment: String,
    pub first_date: String,
    pub last_date: String,
    pub bars: usize,
    pub starting_cash: f64,
    pub fee_per_order: f64,
    pub slippage_bps: f64,
    pub hold_sessions: u32,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub ending_cash: f64,
    pub ending_equity: f64,
    pub open_shares: u64,
    pub completed_trades: usize,
    pub exit_counts: crate::backtest::ExitCounts,
    pub drawdown: crate::risk::DrawdownReport,
    pub buy_hold_drawdown: crate::risk::DrawdownReport,
    pub trade_stats: crate::performance::TradePerformance,
    pub strategy_return_pct: f64,
    pub buy_hold_ending_equity: f64,
    pub buy_hold_return_pct: f64,
    pub buy_hold_shares: u64,
    pub outperformance_pct_points: f64,
}

fn validate_research_costs(
    starting_cash: f64,
    fee_per_order: f64,
    slippage_bps: f64,
) -> Result<(), String> {
    if !starting_cash.is_finite() || !(1.0..=1_000_000_000.0).contains(&starting_cash) {
        return Err("Starting USD must be between 1 and 1,000,000,000".into());
    }
    if !fee_per_order.is_finite() || !(0.0..=10_000.0).contains(&fee_per_order) {
        return Err("Fee per order must be between 0 and 10,000 USD".into());
    }
    if !slippage_bps.is_finite() || !(0.0..=1_000.0).contains(&slippage_bps) {
        return Err("Slippage must be between 0 and 1,000 basis points".into());
    }
    Ok(())
}
fn validate_hold_sessions(hold_sessions: u32) -> Result<(), String> {
    if !(1..=60).contains(&hold_sessions) {
        return Err("Holding period must be between 1 and 60 trading sessions".into());
    }
    Ok(())
}
fn validate_exit_settings(stop_loss_pct: f64, take_profit_pct: f64) -> Result<(), String> {
    if !stop_loss_pct.is_finite()
        || !(0.0..100.0).contains(&stop_loss_pct)
        || !take_profit_pct.is_finite()
        || !(0.0..=1000.0).contains(&take_profit_pct)
    {
        return Err("Stop-loss must be 0-99.999%, take-profit 0-1000%; zero disables".into());
    }
    Ok(())
}

// Preserve existing offline fixture tests and previous default behavior.
#[cfg(test)]
fn summarize_with_hold(
    dataset: HistoryDataset,
    symbol: &str,
    start_date: &str,
    end_date: &str,
    starting_cash: f64,
    fee_per_order: f64,
    slippage_bps: f64,
    hold_sessions: u32,
) -> Result<RealBacktestSummary, String> {
    summarize_with_exits(
        dataset,
        symbol,
        start_date,
        end_date,
        starting_cash,
        fee_per_order,
        slippage_bps,
        hold_sessions,
        0.0,
        0.0,
    )
}

fn summarize_with_exits(
    dataset: HistoryDataset,
    symbol: &str,
    start_date: &str,
    end_date: &str,
    starting_cash: f64,
    fee_per_order: f64,
    slippage_bps: f64,
    hold_sessions: u32,
    stop_loss_pct: f64,
    take_profit_pct: f64,
) -> Result<RealBacktestSummary, String> {
    validate_research_costs(starting_cash, fee_per_order, slippage_bps)?;
    validate_hold_sessions(hold_sessions)?;
    validate_exit_settings(stop_loss_pct, take_profit_pct)?;
    if dataset.symbol != symbol
        || dataset.currency != "USD"
        || dataset.interval != "1day"
        || dataset.adjustment != "splits"
    {
        return Err("Unexpected provider metadata".into());
    }

    let candles = validate_and_sort(dataset.candles)?;
    let first_date = candles.first().unwrap().date.clone();
    let last_date = candles.last().unwrap().date.clone();
    if first_date.as_str() < start_date || last_date.as_str() > end_date {
        return Err("Provider returned prices outside requested dates".into());
    }
    if candles.len() < 50 {
        return Err("Historical series is too short; refusing a partial backtest".into());
    }

    let (result, drawdown) = run_volume_breakout_with_exits(
        &candles,
        starting_cash,
        fee_per_order,
        slippage_bps,
        hold_sessions,
        stop_loss_pct,
        take_profit_pct,
    )?;

    // Keep provider OHLCV in Rust: return aggregate results, never raw candles.
    let (baseline, buy_hold_drawdown) = crate::benchmark::run_buy_and_hold_with_risk(
        &candles,
        starting_cash,
        fee_per_order,
        slippage_bps,
    )?;
    let strategy_return_pct = (result.ending_equity / starting_cash - 1.0) * 100.0;
    let buy_hold_return_pct = (baseline.ending_equity / starting_cash - 1.0) * 100.0;
    let outperformance_pct_points = strategy_return_pct - buy_hold_return_pct;
    let trade_stats = crate::performance::summarize_trades(&result, starting_cash)?;

    Ok(RealBacktestSummary {
        provider: dataset.provider.to_string(),
        symbol: dataset.symbol,
        currency: dataset.currency,
        adjustment: dataset.adjustment.to_string(),
        first_date,
        last_date,
        bars: candles.len(),
        starting_cash,
        fee_per_order,
        slippage_bps,
        hold_sessions,
        stop_loss_pct,
        take_profit_pct,
        ending_cash: result.ending_cash,
        ending_equity: result.ending_equity,
        open_shares: result.open_shares,
        completed_trades: result.trades.len(),
        exit_counts: crate::backtest::summarize_exit_counts(&result),
        drawdown,
        buy_hold_drawdown,
        trade_stats,
        strategy_return_pct,
        buy_hold_ending_equity: baseline.ending_equity,
        buy_hold_return_pct,
        buy_hold_shares: baseline.shares,
        outperformance_pct_points,
    })
}

#[cfg(test)]
fn summarize_with_settings(
    dataset: HistoryDataset,
    symbol: &str,
    start_date: &str,
    end_date: &str,
    starting_cash: f64,
    fee_per_order: f64,
    slippage_bps: f64,
) -> Result<RealBacktestSummary, String> {
    summarize_with_hold(
        dataset,
        symbol,
        start_date,
        end_date,
        starting_cash,
        fee_per_order,
        slippage_bps,
        1,
    )
}
#[cfg(test)]
fn summarize(
    dataset: HistoryDataset,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<RealBacktestSummary, String> {
    summarize_with_settings(
        dataset,
        symbol,
        start_date,
        end_date,
        STARTING_CASH_USD,
        FEE_USD,
        SLIPPAGE_BPS,
    )
}
#[tauri::command]
pub async fn run_real_backtest(
    api_key: String,
    symbol: String,
    start_date: String,
    end_date: String,
    starting_cash: f64,
    fee_per_order: f64,
    slippage_bps: f64,
    hold_sessions: u32,
    stop_loss_pct: f64,
    take_profit_pct: f64,
) -> Result<RealBacktestSummary, String> {
    // Validate all settings before requesting market data.
    validate_research_costs(starting_cash, fee_per_order, slippage_bps)?;
    validate_hold_sessions(hold_sessions)?;
    validate_exit_settings(stop_loss_pct, take_profit_pct)?;
    let symbol = symbol.trim().to_ascii_uppercase();
    let dataset = crate::market_data::fetch_history(
        api_key,
        symbol.clone(),
        start_date.clone(),
        end_date.clone(),
    )
    .await?;
    summarize_with_exits(
        dataset,
        &symbol,
        &start_date,
        &end_date,
        starting_cash,
        fee_per_order,
        slippage_bps,
        hold_sessions,
        stop_loss_pct,
        take_profit_pct,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn validates_research_exit_inputs() {
        assert!(validate_exit_settings(0.0, 0.0).is_ok());
        assert!(validate_exit_settings(5.0, 15.0).is_ok());
        assert!(validate_exit_settings(-1.0, 0.0).is_err());
        assert!(validate_exit_settings(100.0, 0.0).is_err());
        assert!(validate_exit_settings(0.0, 1000.1).is_err());
    }
    #[test]
    fn report_exit_counts_match_completed_trades() {
        let report = summarize_with_hold(
            fixture(),
            "AAPL",
            "2023-01-01",
            "2023-03-31",
            1000.0,
            1.0,
            10.0,
            1,
        )
        .unwrap();
        let count = report.exit_counts;
        assert_eq!(
            count.stop_loss + count.take_profit + count.time_limit,
            report.completed_trades
        );
    }

    #[test]
    fn report_includes_the_buy_and_hold_benchmark() {
        let report = summarize_with_hold(
            fixture(),
            "AAPL",
            "2023-01-01",
            "2023-03-31",
            1000.0,
            1.0,
            10.0,
            1,
        )
        .unwrap();
        assert!((report.strategy_return_pct - 0.0).abs() < 1e-8);
        assert!(report.buy_hold_return_pct > 0.0);
        assert!(report.buy_hold_ending_equity > report.ending_equity);
        assert!(report.outperformance_pct_points < 0.0);
    }

    #[test]
    fn rejects_invalid_research_holding_period() {
        assert!(
            summarize_with_hold(
                fixture(),
                "AAPL",
                "2023-01-01",
                "2023-03-31",
                1000.0,
                1.0,
                10.0,
                0,
            )
            .is_err()
        );
        assert!(
            summarize_with_hold(
                fixture(),
                "AAPL",
                "2023-01-01",
                "2023-03-31",
                1000.0,
                1.0,
                10.0,
                61,
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_invalid_research_costs() {
        assert!(validate_research_costs(0.0, 1.0, 10.0).is_err());
        assert!(validate_research_costs(1000.0, -1.0, 10.0).is_err());
        assert!(validate_research_costs(1000.0, 1.0, f64::NAN).is_err());
    }

    #[test]
    fn uses_custom_starting_capital() {
        let result = summarize_with_settings(
            fixture(),
            "AAPL",
            "2023-01-01",
            "2023-03-31",
            250.0,
            0.0,
            0.0,
        )
        .unwrap();
        assert_eq!(result.starting_cash, 250.0);
        assert_eq!(result.ending_equity, 250.0);
        assert_eq!(result.fee_per_order, 0.0);
        assert_eq!(result.slippage_bps, 0.0);
    }

    use super::*;
    use crate::market_types::Candle;

    fn fixture() -> HistoryDataset {
        let candles = (0..60)
            .map(|i| {
                let (month, day) = if i < 28 {
                    (1, i + 1)
                } else if i < 56 {
                    (2, i - 27)
                } else {
                    (3, i - 55)
                };
                Candle {
                    date: format!("2023-{month:02}-{day:02}"),
                    open: 10.0,
                    high: 12.0,
                    low: 9.0,
                    close: 11.0,
                    volume: 1000,
                }
            })
            .collect();
        HistoryDataset {
            provider: "twelve_data",
            symbol: "AAPL".into(),
            currency: "USD".into(),
            interval: "1day",
            adjustment: "splits",
            candles,
        }
    }

    #[test]
    fn rejects_invalid_provider_metadata() {
        let mut data = fixture();
        data.currency = "EUR".into();
        assert!(summarize(data, "AAPL", "2023-01-01", "2023-03-31").is_err());
    }

    #[test]
    fn summarizes_a_valid_series() {
        let summary = summarize(fixture(), "AAPL", "2023-01-01", "2023-03-31").unwrap();
        assert_eq!(summary.bars, 60);
        assert_eq!(summary.completed_trades, 0);
        assert_eq!(summary.ending_equity, STARTING_CASH_USD);
    }

    #[test]
    fn rejects_incomplete_or_duplicate_series() {
        let mut short = fixture();
        short.candles.truncate(10);
        assert!(summarize(short, "AAPL", "2023-01-01", "2023-03-31").is_err());
        let mut duplicate = fixture();
        duplicate.candles[1].date = duplicate.candles[0].date.clone();
        assert!(summarize(duplicate, "AAPL", "2023-01-01", "2023-03-31").is_err());
    }
}
