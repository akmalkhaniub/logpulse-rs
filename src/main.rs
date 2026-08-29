mod analyzer;
mod parser;
mod report;

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::{Context, Result};
use clap::Parser;
use memmap2::Mmap;

#[derive(Parser, Debug)]
#[command(
    name = "logpulse",
    author = "Akmal Khan <akmalkhaniub>",
    version = "0.1.0",
    about = "⚡ Blazing-fast CLI log and metrics analyzer using zero-copy memory mapping and Rayon parallel map-reduce"
)]
struct Cli {
    /// Path to log file to analyze
    #[arg(short, long, default_value = "sample.log")]
    file: PathBuf,

    /// Number of top endpoints and IPs to display
    #[arg(short, long, default_value_t = 5)]
    top: usize,

    /// Automatically generate a sample 100,000-line test log file and exit
    #[arg(long)]
    generate_sample: bool,
}

fn generate_sample_log_file(path: &PathBuf, count: usize) -> Result<()> {
    println!("Generating synthetic log file ({count} lines) at {}...", path.display());
    let mut file = File::create(path)?;

    let paths = ["/api/v1/users", "/api/v1/orders", "/api/v1/products", "/healthz", "/checkout", "/login", "/search"];
    let ips = ["192.168.1.10", "192.168.1.25", "10.0.0.15", "172.16.0.4", "10.0.0.99"];
    let statuses = [200, 200, 200, 201, 204, 301, 400, 404, 500, 502];

    for i in 0..count {
        let p = paths[i % paths.len()];
        let ip = ips[i % ips.len()];
        let status = statuses[i % statuses.len()];
        let latency = 5.0 + ((i * 37) % 450) as f64 + ((i % 10) as f64 * 0.3);
        let bytes = 120 + ((i * 83) % 4096);

        // Mix between JSON and Common log format
        if i % 2 == 0 {
            writeln!(
                file,
                r#"{{"timestamp":"2026-08-30T02:50:{:02}Z","status":{},"latency_ms":{:.2},"path":"{}","ip":"{}","bytes":{}}}"#,
                i % 60, status, latency, p, ip, bytes
            )?;
        } else {
            writeln!(
                file,
                r#"{} - - [30/Aug/2026:02:50:{:02} +0000] "GET {} HTTP/1.1" {} {} {:.2}"#,
                ip, i % 60, p, status, bytes, latency
            )?;
        }
    }

    println!("✓ Successfully generated sample log file ({} lines)", count);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.generate_sample || (!cli.file.exists() && cli.file.to_str() == Some("sample.log")) {
        generate_sample_log_file(&cli.file, 100_000)?;
        if cli.generate_sample {
            return Ok(());
        }
    }

    let file = File::open(&cli.file)
        .with_context(|| format!("Failed to open log file: {}", cli.file.display()))?;

    let metadata = file.metadata()?;
    let file_size = metadata.len();

    if file_size == 0 {
        println!("File is empty: {}", cli.file.display());
        return Ok(());
    }

    // Zero-copy memory mapping
    let mmap = unsafe { Mmap::map(&file)? };

    let start_time = Instant::now();
    let report = analyzer::analyze_log_bytes(&mmap, cli.top);
    let duration = start_time.elapsed().as_secs_f64();

    report::print_cli_report(
        &report,
        duration,
        cli.file.to_str().unwrap_or("log"),
        file_size,
    );

    Ok(())
}
