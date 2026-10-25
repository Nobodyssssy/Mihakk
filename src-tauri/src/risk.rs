use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DrawdownReport {
    pub max_drawdown_pct: f64,
    pub peak_date: String,
    pub trough_date: String,
}

pub struct DrawdownTracker {
    peak_equity: f64,
    peak_date: String,
    report: DrawdownReport,
}

impl DrawdownTracker {
    pub fn new(starting_cash: f64, first_date: &str) -> Self {
        Self {
            peak_equity: starting_cash,
            peak_date: first_date.into(),
            report: DrawdownReport {
                max_drawdown_pct: 0.0,
                peak_date: first_date.into(),
                trough_date: first_date.into(),
            },
        }
    }

    pub fn observe(&mut self, date: &str, equity: f64) -> Result<(), String> {
        if !equity.is_finite() {
            return Err("Non-finite daily equity".into());
        }
        if equity > self.peak_equity {
            self.peak_equity = equity;
            self.peak_date = date.into();
        } else {
            let decline_pct = 100.0 * (self.peak_equity - equity) / self.peak_equity;
            if decline_pct > self.report.max_drawdown_pct {
                self.report.max_drawdown_pct = decline_pct;
                self.report.peak_date = self.peak_date.clone();
                self.report.trough_date = date.into();
            }
        }
        Ok(())
    }

    pub fn finish(self) -> DrawdownReport {
        self.report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_peak_to_trough_not_total_return() {
        let mut risk = DrawdownTracker::new(100.0, "2023-01-03");
        risk.observe("2023-01-04", 120.0).unwrap();
        risk.observe("2023-01-05", 90.0).unwrap();
        risk.observe("2023-01-06", 110.0).unwrap();
        let report = risk.finish();
        assert!((report.max_drawdown_pct - 25.0).abs() < 1e-8);
        assert_eq!(report.peak_date, "2023-01-04");
        assert_eq!(report.trough_date, "2023-01-05");
    }

    #[test]
    fn resets_peak_after_recovery() {
        let mut risk = DrawdownTracker::new(100.0, "2023-01-03");
        risk.observe("2023-01-04", 80.0).unwrap();
        risk.observe("2023-01-05", 120.0).unwrap();
        risk.observe("2023-01-06", 90.0).unwrap();
        let report = risk.finish();
        assert!((report.max_drawdown_pct - 25.0).abs() < 1e-8);
        assert_eq!(report.peak_date, "2023-01-05");
        assert_eq!(report.trough_date, "2023-01-06");
    }

    #[test]
    fn flat_equity_has_zero_drawdown() {
        let mut risk = DrawdownTracker::new(100.0, "2023-01-03");
        risk.observe("2023-01-04", 100.0).unwrap();
        assert_eq!(risk.finish().max_drawdown_pct, 0.0);
    }

    #[test]
    fn rejects_invalid_equity() {
        let mut risk = DrawdownTracker::new(100.0, "2023-01-03");
        assert!(risk.observe("2023-01-04", f64::NAN).is_err());
    }
}
