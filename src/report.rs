use colored::*;
use crate::analyzer::AnalysisReport;

pub fn print_cli_report(report: &AnalysisReport, duration_secs: f64, file_path: &str, file_size_bytes: u64) {
    let throughput = if duration_secs > 0.0 {
        report.total_requests as f64 / duration_secs
    } else {
        0.0
    };

    let mb_size = file_size_bytes as f64 / (1024.0 * 1024.0);

    println!("\n{}", "=".repeat(75).cyan().bold());
    println!(
        "{} {}",
        "⚡ LOGPULSE METRICS DASHBOARD".bold().bright_white(),
        format!("(Analyzed {:.2} MB in {:.3}s)", mb_size, duration_secs).italic().bright_black()
    );
    println!("File: {}", file_path.yellow());
    println!("{}", "=".repeat(75).cyan().bold());

    // Overview Cards
    println!("\n{}", "📊 TRAFFIC SUMMARY".bold().underline());
    println!(
        "  • Total Requests:     {} (Throughput: {} req/s)",
        report.total_requests.to_string().green().bold(),
        format!("{:.0}", throughput).bright_green().bold()
    );
    println!(
        "  • Total Data Volume:  {:.2} MB",
        report.total_bytes as f64 / (1024.0 * 1024.0)
    );

    let error_str = format!("{:.2}%", report.error_rate_pct);
    let colored_err = if report.error_rate_pct > 5.0 {
        error_str.red().bold()
    } else if report.error_rate_pct > 0.0 {
        error_str.yellow().bold()
    } else {
        error_str.green().bold()
    };
    println!("  • Error Rate:         {}", colored_err);

    // Status Code Breakdown
    println!("\n{}", "📈 HTTP STATUS DISTRIBUTION".bold().underline());
    println!("  • 2xx (Success):     {} ({:.1}%)", report.status_2xx.to_string().green(), pct(report.status_2xx, report.total_requests));
    println!("  • 3xx (Redirect):    {} ({:.1}%)", report.status_3xx.to_string().blue(), pct(report.status_3xx, report.total_requests));
    println!("  • 4xx (Client Err):  {} ({:.1}%)", report.status_4xx.to_string().yellow(), pct(report.status_4xx, report.total_requests));
    println!("  • 5xx (Server Err):  {} ({:.1}%)", report.status_5xx.to_string().red(), pct(report.status_5xx, report.total_requests));

    // Latency Percentiles
    println!("\n{}", "⏱️  LATENCY PERCENTILES (ms)".bold().underline());
    println!(
        "  • Min: {:>6.2}ms  |  Mean: {:>6.2}ms  |  Max: {:>6.2}ms",
        report.min_latency_ms, report.mean_latency_ms, report.max_latency_ms
    );
    println!(
        "  • p50 (Median): {:>6.2}ms  |  p90: {:>6.2}ms  |  p95: {:>6.2}ms  |  p99: {:>6.2}ms",
        report.p50_latency_ms.to_string().green(),
        report.p90_latency_ms.to_string().bright_green(),
        report.p95_latency_ms.to_string().yellow(),
        report.p99_latency_ms.to_string().red().bold()
    );

    // Top Paths Table
    println!("\n{}", "🔥 TOP REQUESTED ENDPOINTS".bold().underline());
    for (i, (path, count)) in report.top_paths.iter().enumerate() {
        println!(
            "  {:>2}. {:<45} {:>8} hits ({:.1}%)",
            i + 1,
            path.bright_white(),
            count.to_string().cyan(),
            pct(*count, report.total_requests)
        );
    }

    // Top Client IPs
    println!("\n{}", "🌐 TOP CLIENT IP ADDRESSES".bold().underline());
    for (i, (ip, count)) in report.top_ips.iter().enumerate() {
        println!(
            "  {:>2}. {:<20} {:>8} requests ({:.1}%)",
            i + 1,
            ip.bright_yellow(),
            count.to_string().cyan(),
            pct(*count, report.total_requests)
        );
    }

    println!("\n{}", "=".repeat(75).cyan().bold());
}

fn pct(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}
