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
pub struct HistoryDataset {
    pub provider: &'static str,
    pub symbol: String,
    pub currency: String,
    pub interval: &'static str,
    pub adjustment: &'static str,
    pub candles: Vec<crate::market_types::Candle>,
}

fn parse_history(json: &str) -> Result<HistoryDataset, String> {
    let data: ApiResponse = serde_json::from_str(json).map_err(|_| "Invalid API response")?;

    if data.status != "ok" {
        return Err(data.message.unwrap_or_else(|| "Provider error".into()));
    }

    let meta = data.meta.ok_or("Missing symbol information")?;
    let bars = data.values.ok_or("Missing historical prices")?;

    let candles: Vec<crate::market_types::Candle> = bars
        .into_iter()
        .map(|bar| {
            let price = |value: &str| {
                value
                    .parse::<f64>()
                    .map_err(|_| "Invalid price".to_string())
            };

            Ok(crate::market_types::Candle {
                date: bar.datetime,
                open: price(&bar.open)?,
                high: price(&bar.high)?,
                low: price(&bar.low)?,
                close: price(&bar.close)?,
                volume: bar
                    .volume
                    .parse::<u64>()
                    .map_err(|_| "Invalid volume".to_string())?,
            })
        })
        .collect::<Result<_, String>>()?;

    let candles = crate::market_types::validate_and_sort(candles)?;

    Ok(HistoryDataset {
        provider: "twelve_data",
        symbol: meta.symbol,
        currency: meta.currency,
        interval: "1day",
        adjustment: "splits",
        candles,
    })
}
#[tauri::command]
pub async fn fetch_history(api_key: String) -> Result<HistoryDataset, String> {
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

    parse_history(&body)
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

        let result = parse_history(json).unwrap();
        assert_eq!(result.candles.len(), 2);
        assert_eq!(result.candles[0].date, "2023-01-03");
        assert_eq!(result.candles[1].date, "2023-01-04");
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

        assert!(parse_history(json).is_err());
    }
}
