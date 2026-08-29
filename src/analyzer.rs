use std::collections::HashMap;
use rayon::prelude::*;
use crate::parser::{parse_log_line, ParsedLogRecord};

#[derive(Debug, Default, Clone)]
pub struct AggregatedStats {
    pub total_requests: u64,
    pub total_bytes: u64,
    pub status_2xx: u64,
    pub status_3xx: u64,
    pub status_4xx: u64,
    pub status_5xx: u64,
    pub path_counts: HashMap<String, u64>,
    pub ip_counts: HashMap<String, u64>,
    pub latencies: Vec<f64>,
}

impl AggregatedStats {
    pub fn merge(&mut self, other: AggregatedStats) {
        self.total_requests += other.total_requests;
        self.total_bytes += other.total_bytes;
        self.status_2xx += other.status_2xx;
        self.status_3xx += other.status_3xx;
        self.status_4xx += other.status_4xx;
        self.status_5xx += other.status_5xx;

        for (path, count) in other.path_counts {
            *self.path_counts.entry(path).or_insert(0) += count;
        }

        for (ip, count) in other.ip_counts {
            *self.ip_counts.entry(ip).or_insert(0) += count;
        }

        self.latencies.extend(other.latencies);
    }

    pub fn record(&mut self, record: ParsedLogRecord) {
        self.total_requests += 1;
        self.total_bytes += record.bytes;

        match record.status {
            200..=299 => self.status_2xx += 1,
            300..=399 => self.status_3xx += 1,
            400..=499 => self.status_4xx += 1,
            500..=599 => self.status_5xx += 1,
            _ => (),
        }

        *self.path_counts.entry(record.path).or_insert(0) += 1;
        *self.ip_counts.entry(record.ip).or_insert(0) += 1;

        if record.latency_ms > 0.0 {
            self.latencies.push(record.latency_ms);
        }
    }
}

#[derive(Debug)]
pub struct AnalysisReport {
    pub total_requests: u64,
    pub total_bytes: u64,
    pub error_rate_pct: f64,
    pub status_2xx: u64,
    pub status_3xx: u64,
    pub status_4xx: u64,
    pub status_5xx: u64,
    pub min_latency_ms: f64,
    pub mean_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub max_latency_ms: f64,
    pub top_paths: Vec<(String, u64)>,
    pub top_ips: Vec<(String, u64)>,
}

/// Analyzes an in-memory or memory-mapped UTF-8 byte slice using parallel chunk reduction.
pub fn analyze_log_bytes(bytes: &[u8], top_k: usize) -> AnalysisReport {
    let content = String::from_utf8_lossy(bytes);

    // Process lines in parallel chunks
    let mut aggregated: AggregatedStats = content
        .par_lines()
        .fold(AggregatedStats::default, |mut acc, line| {
            if let Some(rec) = parse_log_line(line) {
                acc.record(rec);
            }
            acc
        })
        .reduce(AggregatedStats::default, |mut a, b| {
            a.merge(b);
            a
        });

    let total = aggregated.total_requests;
    let errors = aggregated.status_4xx + aggregated.status_5xx;
    let error_rate_pct = if total > 0 {
        (errors as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Calculate latency percentiles
    aggregated.latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n_lat = aggregated.latencies.len();

    let (min_l, mean_l, p50, p90, p95, p99, max_l) = if n_lat > 0 {
        let sum: f64 = aggregated.latencies.iter().sum();
        let mean = sum / n_lat as f64;
        let min = aggregated.latencies[0];
        let max = aggregated.latencies[n_lat - 1];
        let p50 = aggregated.latencies[(n_lat as f64 * 0.50) as usize];
        let p90 = aggregated.latencies[((n_lat as f64 * 0.90) as usize).min(n_lat - 1)];
        let p95 = aggregated.latencies[((n_lat as f64 * 0.95) as usize).min(n_lat - 1)];
        let p99 = aggregated.latencies[((n_lat as f64 * 0.99) as usize).min(n_lat - 1)];
        (min, mean, p50, p90, p95, p99, max)
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    };

    // Top K Paths
    let mut path_vec: Vec<(String, u64)> = aggregated.path_counts.into_iter().collect();
    path_vec.sort_by(|a, b| b.1.cmp(&a.1));
    path_vec.truncate(top_k);

    // Top K IPs
    let mut ip_vec: Vec<(String, u64)> = aggregated.ip_counts.into_iter().collect();
    ip_vec.sort_by(|a, b| b.1.cmp(&a.1));
    ip_vec.truncate(top_k);

    AnalysisReport {
        total_requests: total,
        total_bytes: aggregated.total_bytes,
        error_rate_pct,
        status_2xx: aggregated.status_2xx,
        status_3xx: aggregated.status_3xx,
        status_4xx: aggregated.status_4xx,
        status_5xx: aggregated.status_5xx,
        min_latency_ms: min_l,
        mean_latency_ms: mean_l,
        p50_latency_ms: p50,
        p90_latency_ms: p90,
        p95_latency_ms: p95,
        p99_latency_ms: p99,
        max_latency_ms: max_l,
        top_paths: path_vec,
        top_ips: ip_vec,
    }
}
