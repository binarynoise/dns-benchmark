use crate::config::AppConfig;
use crate::dns_server::DnsServer;
use chrono::prelude::{DateTime, Utc};
use csv::QuoteStyle;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::ProtoErrorKind;
use hickory_resolver::{ResolveError, ResolveErrorKind, Resolver};
use indexmap::IndexMap;
use rand::random;
use sprintf::sprintf;
use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::time::{Duration, SystemTime};

mod config;
mod dns_server;

#[tokio::main]
async fn main() {
    let timestamp = timestamp(SystemTime::now());
    let config = AppConfig::load().unwrap();

    println!("{:?}", config);

    let mut domains: Vec<String> = Vec::new();
    if let Some(fixed) = config.domain.fixed {
        for _ in 0..config.domain.repeat {
            domains.push(fixed.clone());
        }
    }
    if let Some(format) = config.domain.format {
        let prefix: u32 = random();
        for i in 0..config.domain.repeat {
            domains.push(sprintf!(format.as_str(), prefix, i).unwrap());
        }
    }
    if let Some(file) = config.domain.file {
        let file = File::open(&file).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map(|line| line.unwrap()).collect();
        for _ in 0..config.domain.repeat {
            domains.extend(lines.clone());
        }
    }
    for domain in &mut domains {
        if !domain.ends_with(".") {
            domain.push('.');
        }
    }
    println!("Loaded {} domains", domains.len());
    assert!(!domains.is_empty());

    // dns

    let servers: Vec<DnsServer> = config
        .runs
        .dns
        .servers
        .into_iter()
        .map(DnsServer::new_dns)
        .collect::<Vec<_>>();

    println!("dns: benchmarking {} servers", servers.len());
    let results = run_benchmark(&domains, servers).await;
    println!();
    display_results(&timestamp, "dns", results);

    // dot

    let system_resolver = Resolver::builder(TokioConnectionProvider::default())
        .unwrap()
        .build();
    let servers: Vec<DnsServer> = FuturesOrdered::from_iter(
        config
            .runs
            .dot
            .servers
            .into_iter()
            .map(|server| DnsServer::new_dot(server, &system_resolver))
            .collect::<Vec<_>>(),
    )
    .collect()
    .await;
    println!("dot: benchmarking {} servers", servers.len());
    let results = run_benchmark(&domains, servers).await;
    println!();
    display_results(&timestamp, "dot", results);
}

fn display_results<R: Display>(
    timestamp: &String,
    name: &str,
    results: IndexMap<R, Vec<Duration>>,
) {
    std::fs::create_dir_all("results").unwrap();
    let filename = format!("results/dns-benchmark-{timestamp}-{name}.csv");
    let mut csv = csv::WriterBuilder::default()
        .quote_style(QuoteStyle::NonNumeric)
        .from_path(&filename)
        .unwrap();

    let headers: Vec<String> = results.keys().map(|x| format!("{x}")).collect();
    csv.write_record(&headers).expect("Failed to write header");

    let max_len = results.values().map(|v| v.len()).max().unwrap_or(0);
    for i in 0..max_len {
        let mut record = Vec::new();
        for durations in results.values() {
            if let Some(duration) = durations.get(i) {
                record.push(format!("{}", duration.as_millis()));
            } else {
                record.push(String::new());
            }
        }
        csv.write_record(&record).expect("Failed to write record");
    }

    println!("Results saved to {}", filename);
}

async fn run_benchmark(
    domains: &Vec<String>,
    servers: Vec<DnsServer>,
) -> IndexMap<DnsServer, Vec<Duration>> {
    let mut results: IndexMap<DnsServer, Vec<Duration>> = servers
        .into_iter()
        .map(|dns_server| (dns_server, Vec::<Duration>::new()))
        .collect();

    for domain in domains {
        for (server, result) in results.iter_mut() {
            let (r, time) = measure_time_async(async || server.resolve4(domain).await).await;
            process_result(server, result, r, time);
            let (r, time) = measure_time_async(async || server.resolve6(domain).await).await;
            process_result(server, result, r, time);
        }
        print!(".");
        io::stdout().flush().unwrap();
    }
    results
}

fn process_result<S: Display, R>(
    server: &S,
    result: &mut Vec<Duration>,
    r: Result<R, ResolveError>,
    time: Duration,
) {
    match r {
        Ok(_) => result.push(time),
        Err(e) => {
            if let ResolveErrorKind::Proto(pe) = e.kind() {
                match pe.kind.as_ref() {
                    ProtoErrorKind::NoRecordsFound { .. } => {
                        result.push(time);
                        return;
                    }
                    ProtoErrorKind::Timeout => {
                        result.push(Duration::from_secs(10));
                        return;
                    }
                    _ => {}
                };
            };
            result.push(Duration::from_secs(20));
            eprintln!("\n{}: {:?}", server, e);
        }
    }
}

async fn measure_time_async<F, R, Fut>(f: F) -> (R, Duration)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = R>,
{
    let start = SystemTime::now();
    let r = f().await;
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();
    (r, duration)
}

fn timestamp(st: SystemTime) -> String {
    let dt: DateTime<Utc> = st.into();
    format!("{}", dt.format("%Y-%m-%d_%H:%M:%S"))
}
