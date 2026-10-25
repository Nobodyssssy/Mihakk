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

// Validate an actual Gregorian calendar date, not just YYYY-MM-DD shape.
// Reused by API request validation and every OHLCV dataset.
pub(crate) fn valid_calendar_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return false;
    }

    let year: u32 = value[0..4].parse().unwrap_or(0);
    let month: u32 = value[5..7].parse().unwrap_or(0);
    let day: u32 = value[8..10].parse().unwrap_or(0);
    if !(1900..=2100).contains(&year) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

pub fn validate_and_sort(mut candles: Vec<Candle>) -> Result<Vec<Candle>, String> {
    if candles.is_empty() {
        return Err("No historical candles".into());
    }

    for candle in &candles {
        if !valid_calendar_date(&candle.date) {
            return Err("Invalid trading date".into());
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
    fn rejects_nonexistent_calendar_dates() {
        for date in [
            "2023-02-29",
            "2024-02-30",
            "2023-04-31",
            "2023-13-01",
            "2023-00-10",
            "2023-01-00",
            "2023-01-32",
            "2023-1-01",
        ] {
            assert!(!valid_calendar_date(date), "accepted {date}");
            assert!(validate_and_sort(vec![candle(date)]).is_err());
        }
    }

    #[test]
    fn accepts_real_leap_day() {
        let data = validate_and_sort(vec![candle("2024-02-29")]).unwrap();
        assert_eq!(data[0].date, "2024-02-29");
    }

    #[test]
    fn rejects_non_finite_ohlc() {
        let mut bad = candle("2023-01-03");
        bad.close = f64::NAN;
        assert!(validate_and_sort(vec![bad.clone()]).is_err());
        bad.close = f64::INFINITY;
        assert!(validate_and_sort(vec![bad]).is_err());
    }

    #[test]
    fn rejects_invalid_prices() {
        let mut bad = candle("2023-01-03");
        bad.high = 8.0;
        assert!(validate_and_sort(vec![bad]).is_err());
    }
}
