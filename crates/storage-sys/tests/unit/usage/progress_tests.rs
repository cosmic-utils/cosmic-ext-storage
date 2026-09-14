use std::time::{Duration, Instant};

use super::{
    compute_eta, compute_eta_from_throughput, compute_progress_percent, ewma_update, format_eta,
};

#[test]
fn percent_and_eta_handle_zero_denominator() {
    let started = Instant::now() - Duration::from_secs(5);
    assert_eq!(compute_progress_percent(1024, 0), 0.0);
    assert!(compute_eta(1024, 0, started).is_none());
    assert!(compute_eta_from_throughput(1024, 0, 1000.0).is_none());
    assert_eq!(ewma_update(None, 100.0, 0.2), 100.0);
    assert_eq!(ewma_update(Some(100.0), 200.0, 0.2), 120.0);
    assert_eq!(format_eta(None), "--:--:--");
}
