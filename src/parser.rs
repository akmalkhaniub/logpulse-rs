use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct JsonLogLine {
    pub status: Option<u16>,
    pub status_code: Option<u16>,
    pub latency_ms: Option<f64>,
    pub response_time: Option<f64>,
    pub duration_ms: Option<f64>,
    pub path: Option<String>,
    pub uri: Option<String>,
    pub url: Option<String>,
    pub ip: Option<String>,
    pub client_ip: Option<String>,
    pub bytes: Option<u64>,
    pub bytes_sent: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedLogRecord {
    pub status: u16,
    pub latency_ms: f64,
    pub path: String,
    pub ip: String,
    pub bytes: u64,
}

pub enum LogFormat {
    Auto,
    Json,
    Common,
}

/// Parses a single line from a log file into a structured record.
pub fn parse_log_line(line: &str) -> Option<ParsedLogRecord> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Try parsing as JSON first
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        if let Ok(json_line) = serde_json::from_str::<JsonLogLine>(trimmed) {
            let status = json_line.status.or(json_line.status_code).unwrap_or(200);
            let latency_ms = json_line
                .latency_ms
                .or(json_line.response_time)
                .or(json_line.duration_ms)
                .unwrap_or(0.0);
            let path = json_line
                .path
                .or(json_line.uri)
                .or(json_line.url)
                .unwrap_or_else(|| "/".to_string());
            let ip = json_line
                .ip
                .or(json_line.client_ip)
                .unwrap_or_else(|| "127.0.0.1".to_string());
            let bytes = json_line.bytes.or(json_line.bytes_sent).unwrap_or(0);

            return Some(ParsedLogRecord {
                status,
                latency_ms,
                path,
                ip,
                bytes,
            });
        }
    }

    // Fallback: parse Standard Web Server Common/Combined log line
    // Format: <ip> - - [<timestamp>] "<METHOD> <path> <proto>" <status> <bytes> [latency_ms]
    parse_common_log_line(trimmed)
}

fn parse_common_log_line(line: &str) -> Option<ParsedLogRecord> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }

    let ip = parts[0].to_string();

    // Find HTTP request quotes
    let quote_start = line.find('"')?;
    let quote_end = line[quote_start + 1..].find('"')? + quote_start + 1;
    let request_str = &line[quote_start + 1..quote_end];
    let req_parts: Vec<&str> = request_str.split_whitespace().collect();

    let path = if req_parts.len() >= 2 {
        req_parts[1].to_string()
    } else {
        "/".to_string()
    };

    let remainder = line[quote_end + 1..].trim();
    let rem_parts: Vec<&str> = remainder.split_whitespace().collect();

    if rem_parts.is_empty() {
        return None;
    }

    let status = rem_parts[0].parse::<u16>().unwrap_or(200);
    let bytes = if rem_parts.len() > 1 {
        rem_parts[1].parse::<u64>().unwrap_or(0)
    } else {
        0
    };

    let latency_ms = if rem_parts.len() > 2 {
        rem_parts[2].parse::<f64>().unwrap_or(0.0)
    } else {
        0.0
    };

    Some(ParsedLogRecord {
        status,
        latency_ms,
        path,
        ip,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        let line = r#"{"status": 500, "latency_ms": 124.5, "path": "/api/checkout", "ip": "10.0.0.1", "bytes": 512}"#;
        let record = parse_log_line(line).expect("must parse JSON line");
        assert_eq!(record.status, 500);
        assert_eq!(record.latency_ms, 124.5);
        assert_eq!(record.path, "/api/checkout");
        assert_eq!(record.ip, "10.0.0.1");
        assert_eq!(record.bytes, 512);
    }

    #[test]
    fn test_parse_common() {
        let line = r#"192.168.1.100 - - [30/Aug/2026:02:50:00 +0000] "GET /healthz HTTP/1.1" 200 45 3.2"#;
        let record = parse_log_line(line).expect("must parse combined line");
        assert_eq!(record.status, 200);
        assert_eq!(record.path, "/healthz");
        assert_eq!(record.ip, "192.168.1.100");
        assert_eq!(record.bytes, 45);
        assert_eq!(record.latency_ms, 3.2);
    }
}
