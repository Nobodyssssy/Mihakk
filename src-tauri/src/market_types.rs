use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Candle {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

pub fn validate_and_sort(mut candles: Vec<Candle>) -> Result<Vec<Candle>, String> {
    if candles.is_empty() {
        return Err("No historical candles".into());
    }

    for candle in &candles {
        let date = candle.date.as_bytes();
        if date.len() != 10
            || date[4] != b'-'
            || date[7] != b'-'
            || !date
                .iter()
                .enumerate()
                .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        {
            return Err("Invalid date format".into());
        }

        let prices = [candle.open, candle.high, candle.low, candle.close];

        if prices.iter().any(|p| !p.is_finite() || *p <= 0.0) {
            return Err("Invalid price".into());
        }

        if candle.high < candle.open.max(candle.close)
            || candle.low > candle.open.min(candle.close)
            || candle.high < candle.low
        {
            return Err("Invalid OHLC candle".into());
        }
    }

    candles.sort_by(|a, b| a.date.cmp(&b.date));

    if candles.windows(2).any(|pair| pair[0].date == pair[1].date) {
        return Err("Duplicate trading date".into());
    }

    Ok(candles)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(date: &str) -> Candle {
        Candle {
            date: date.into(),
            open: 10.0,
            high: 12.0,
            low: 9.0,
            close: 11.0,
            volume: 1000,
        }
    }

    #[test]
    fn sorts_candles_chronologically() {
        let data = validate_and_sort(vec![candle("2023-01-04"), candle("2023-01-03")]).unwrap();

        assert_eq!(data[0].date, "2023-01-03");
    }

    #[test]
    fn rejects_duplicate_dates() {
        assert!(validate_and_sort(vec![candle("2023-01-03"), candle("2023-01-03"),]).is_err());
    }

    #[test]
    fn rejects_invalid_prices() {
        let mut bad = candle("2023-01-03");
        bad.high = 8.0;
        assert!(validate_and_sort(vec![bad]).is_err());
    }
}
