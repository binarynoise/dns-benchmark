mod benchmark;
mod config;
mod dns_server;
mod domains;
mod results;

use crate::benchmark::run_benchmark;
use crate::config::AppConfig;
use crate::domains::load_domains;
use chrono::{DateTime, Utc};
use results::save_results_to_csv;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timestamp_str = timestamp(SystemTime::now());
    let config = AppConfig::load().map_err(|err| format!("Could not load config: {err}"))?;

    let domains = load_domains(&config.domain)?;
    assert!(!domains.is_empty());

    let benchmark_results = run_benchmark(&config, domains).await?;

    save_results_to_csv(&benchmark_results.results, &timestamp_str)?;
    Ok(())
}

fn timestamp(st: SystemTime) -> String {
    let dt: DateTime<Utc> = st.into();
    dt.format("%Y-%m-%d_%H:%M:%S").to_string()
}
