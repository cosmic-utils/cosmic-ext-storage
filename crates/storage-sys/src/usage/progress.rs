// SPDX-License-Identifier: GPL-3.0-only

use std::time::{Duration, Instant};

pub fn compute_progress_percent(bytes_processed: u64, estimated_total_used_bytes: u64) -> f64 {
    if estimated_total_used_bytes == 0 {
        return 0.0;
    }

    let ratio = bytes_processed as f64 / estimated_total_used_bytes as f64;
    (ratio * 100.0).clamp(0.0, 100.0)
}

pub fn compute_eta(
    bytes_processed: u64,
    estimated_total_used_bytes: u64,
    started_at: Instant,
) -> Option<Duration> {
    if estimated_total_used_bytes == 0 || bytes_processed == 0 {
        return None;
    }

    if bytes_processed >= estimated_total_used_bytes {
        return Some(Duration::from_secs(0));
    }

    let elapsed = started_at.elapsed();
    if elapsed.is_zero() {
        return None;
    }

    let throughput = bytes_processed as f64 / elapsed.as_secs_f64();
    compute_eta_from_throughput(bytes_processed, estimated_total_used_bytes, throughput)
}

pub fn compute_eta_from_throughput(
    bytes_processed: u64,
    estimated_total_used_bytes: u64,
    throughput_bytes_per_sec: f64,
) -> Option<Duration> {
    if estimated_total_used_bytes == 0 || bytes_processed == 0 {
        return None;
    }

    if bytes_processed >= estimated_total_used_bytes {
        return Some(Duration::from_secs(0));
    }

    if throughput_bytes_per_sec <= 0.0 {
        return None;
    }

    let remaining = (estimated_total_used_bytes - bytes_processed) as f64;
    let eta_seconds = (remaining / throughput_bytes_per_sec).max(0.0);

    Some(Duration::from_secs_f64(eta_seconds))
}

pub fn ewma_update(previous: Option<f64>, sample: f64, alpha: f64) -> f64 {
    let alpha = alpha.clamp(0.0, 1.0);
    match previous {
        Some(prev) => alpha * sample + (1.0 - alpha) * prev,
        None => sample,
    }
}

pub fn format_eta(eta: Option<Duration>) -> String {
    let Some(eta) = eta else {
        return "--:--:--".to_string();
    };

    let total_secs = eta.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut unit_index = 0;
    let mut value = bytes as f64;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
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
}
