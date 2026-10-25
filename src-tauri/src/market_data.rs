use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize)]
struct ApiBar {
    datetime: String,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
}

#[derive(Deserialize)]
struct ApiMeta {
    symbol: String,
    currency: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    status: String,
    message: Option<String>,
    meta: Option<ApiMeta>,
    values: Option<Vec<ApiBar>>,
}

#[derive(Serialize)]
pub struct HistorySummary {
    symbol: String,
    currency: String,
    bars: usize,
    first_date: String,
    last_date: String,
}

fn parse_summary(json: &str) -> Result<HistorySummary, String> {
    let data: ApiResponse = serde_json::from_str(json).map_err(|_| "Invalid API response")?;

    if data.status != "ok" {
        return Err(data.message.unwrap_or_else(|| "Provider error".into()));
    }

    let meta = data.meta.ok_or("Missing symbol information")?;
    let bars = data.values.ok_or("Missing historical prices")?;

    if bars.is_empty() {
        return Err("No historical candles returned".into());
    }

    for bar in &bars {
        let price = |v: &str| -> Result<f64, String> {
            let n: f64 = v.parse().map_err(|_| "Invalid price")?;
            if !n.is_finite() || n <= 0.0 {
                return Err("Non-positive or non-finite price".into());
            }
            Ok(n)
        };

        let o = price(&bar.open)?;
        let h = price(&bar.high)?;
        let l = price(&bar.low)?;
        let c = price(&bar.close)?;

        if h < o.max(c) || l > o.min(c) || h < l {
            return Err("Invalid OHLC candle".into());
        }

        bar.volume
            .parse::<u64>()
            .map_err(|_| "Invalid trading volume")?;

        if bar.datetime.len() != 10 {
            return Err("Invalid trading date".into());
        }
    }

    let first = bars
        .iter()
        .map(|b| b.datetime.as_str())
        .min()
        .unwrap()
        .to_string();
    let last = bars
        .iter()
        .map(|b| b.datetime.as_str())
        .max()
        .unwrap()
        .to_string();

    Ok(HistorySummary {
        symbol: meta.symbol,
        currency: meta.currency,
        bars: bars.len(),
        first_date: first,
        last_date: last,
    })
}

#[tauri::command]
pub async fn fetch_history_summary(api_key: String) -> Result<HistorySummary, String> {
    if api_key.trim().is_empty() {
        return Err("API key is required".into());
    }

    let response = reqwest::Client::new()
        .get("https://api.twelvedata.com/time_series")
        .header("Authorization", format!("apikey {}", api_key.trim()))
        .query(&[
            ("symbol", "AAPL"),
            ("interval", "1day"),
            ("start_date", "2023-01-01"),
            ("end_date", "2023-03-31"),
            ("adjust", "splits"),
        ])
        .timeout(Duration::from_secs(25))
        .send()
        .await
        .map_err(|_| "Market data request failed")?
        .error_for_status()
        .map_err(|_| "Market data HTTP error")?;

    let body = response
        .text()
        .await
        .map_err(|_| "Could not read provider response")?;

    parse_summary(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_history_is_summarized() {
        let json = r#"{
            "status":"ok",
            "meta":{"symbol":"AAPL","currency":"USD"},
            "values":[
                {"datetime":"2023-01-04","open":"11","high":"13",
                 "low":"10","close":"12","volume":"1000"},
                {"datetime":"2023-01-03","open":"10","high":"12",
                 "low":"9","close":"11","volume":"900"}
            ]
        }"#;

        let result = parse_summary(json).unwrap();
        assert_eq!(result.bars, 2);
        assert_eq!(result.first_date, "2023-01-03");
        assert_eq!(result.last_date, "2023-01-04");
    }

    #[test]
    fn invalid_candle_is_rejected() {
        let json = r#"{
            "status":"ok",
            "meta":{"symbol":"AAPL","currency":"USD"},
            "values":[
                {"datetime":"2023-01-03","open":"10","high":"8",
                 "low":"9","close":"11","volume":"900"}
            ]
        }"#;

        assert!(parse_summary(json).is_err());
    }
}
