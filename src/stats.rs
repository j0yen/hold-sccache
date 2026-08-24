use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct SccacheStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f64,
    pub cache_size: u64,
    pub max_cache_size: u64,
}

pub(crate) fn parse_stats(text: &str) -> SccacheStats {
    let cache_hits = extract_count(text, "Cache hits").unwrap_or(0);
    let cache_misses = extract_count(text, "Cache misses").unwrap_or(0);
    let cache_size = extract_size_bytes(text, "Cache size").unwrap_or(0);
    let max_cache_size = extract_size_bytes(text, "Max cache size").unwrap_or(0);

    let total = cache_hits + cache_misses;
    let hit_rate = if total == 0 {
        0.0_f64
    } else {
        #[allow(clippy::cast_precision_loss)]
        let r = cache_hits as f64 / total as f64;
        r
    };

    SccacheStats {
        cache_hits,
        cache_misses,
        hit_rate,
        cache_size,
        max_cache_size,
    }
}

pub(crate) fn show_stats(fixture: Option<&str>) -> Result<()> {
    let text = if let Some(path) = fixture {
        std::fs::read_to_string(path)
            .with_context(|| format!("reading fixture {path}"))?
    } else {
        let output = std::process::Command::new("sccache")
            .arg("--show-stats")
            .output()
            .context("running sccache --show-stats; is sccache installed and on PATH?")?;
        String::from_utf8_lossy(&output.stdout).into_owned()
    };

    let stats = parse_stats(&text);
    let json = serde_json::to_string_pretty(&stats).context("serializing stats to JSON")?;
    println!("{json}");
    Ok(())
}

fn extract_count(text: &str, key: &str) -> Option<u64> {
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        if let Ok(n) = parts.last()?.parse::<u64>() {
            let line_key = parts[..parts.len() - 1].join(" ");
            if line_key == key {
                return Some(n);
            }
        }
    }
    None
}

fn extract_size_bytes(text: &str, key: &str) -> Option<u64> {
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let unit = parts[parts.len() - 1];
        if let Ok(n) = parts[parts.len() - 2].parse::<f64>() {
            let line_key = parts[..parts.len() - 2].join(" ");
            if line_key == key {
                let multiplier: f64 = match unit {
                    "GiB" => 1024.0 * 1024.0 * 1024.0,
                    "MiB" => 1024.0 * 1024.0,
                    "KiB" => 1024.0,
                    "B" => 1.0,
                    _ => return None,
                };
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                return Some((n * multiplier) as u64);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"Compile requests                      100
Compile requests executed              98
Cache hits                             80
Cache misses                           18
Cache hits (C/C++)                      0
Cache hits (Rust)                      80
Cache misses (Rust)                    18
Cache timeouts                          0
Cache read errors                       0
Forced recaches                         0
Cache write errors                      0
Compilations                            0
Errors                                  0
Cache location                       Local disk: "/home/user/.cache/sccache"
Cache size                            2.1 GiB
Max cache size                       20.0 GiB
"#;

    #[test]
    fn test_parse_cache_hits() {
        let stats = parse_stats(FIXTURE);
        assert_eq!(stats.cache_hits, 80);
    }

    #[test]
    fn test_parse_cache_misses() {
        let stats = parse_stats(FIXTURE);
        assert_eq!(stats.cache_misses, 18);
    }

    #[test]
    fn test_parse_hit_rate_in_range() {
        let stats = parse_stats(FIXTURE);
        let expected = 80.0_f64 / 98.0_f64;
        assert!((stats.hit_rate - expected).abs() < 0.001);
    }

    #[test]
    fn test_parse_cache_size() {
        let stats = parse_stats(FIXTURE);
        assert!(stats.cache_size > 0);
    }

    #[test]
    fn test_parse_max_size() {
        let stats = parse_stats(FIXTURE);
        let expected = (20.0_f64 * 1024.0 * 1024.0 * 1024.0) as u64;
        assert_eq!(stats.max_cache_size, expected);
    }

    #[test]
    fn test_parse_empty_stats() {
        let stats = parse_stats("");
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.cache_size, 0);
        assert_eq!(stats.max_cache_size, 0);
    }
}
