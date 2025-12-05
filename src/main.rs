use crate::config::AppConfig;
use crate::dns_server::DnsServer;
use chrono::prelude::{DateTime, Utc};
use csv::QuoteStyle;
use futures::stream::{FuturesUnordered, StreamExt};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::ProtoErrorKind;
use hickory_resolver::{ResolveError, ResolveErrorKind, Resolver};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use indexmap::IndexMap;
use rand::seq::SliceRandom;
use rand::{random, Rng};
use sprintf::sprintf;
use std::borrow::Cow;
use std::cmp::min;
use std::fmt::Display;
use std::fs::File;
use std::future::Future;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

mod config;
mod dns_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = timestamp(SystemTime::now());
    let config = AppConfig::load().map_err(|err| format!("Could not load config: {err}"))?;

    #[cfg(debug_assertions)]
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
            domains.push(sprintf!(format.as_str(), prefix, i)?);
        }
    }
    let mut rng = rand::rng();
    if let Some(file) = config.domain.file {
        let file = File::open(&file).map_err(|err| format!("Could not open file {file}: {err}"))?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().skip(1).collect::<Result<_, _>>()?;
        for _ in 0..config.domain.repeat {
            domains = lines.clone();
            domains.shuffle(&mut rng);
        }
    }
    for domain in &mut domains {
        if !domain.ends_with(".") {
            domain.push('.');
        }
    }
    let domains = domains; // make readonly

    println!("Loaded {} domains", domains.len());
    assert!(!domains.is_empty());

    let system_resolver = Resolver::builder(TokioConnectionProvider::default())?.build();

    let mut results: IndexMap<DnsServer, (Vec<Duration>, Vec<String>)> = IndexMap::new();

    for group in config.groups {
        let mut servers: Vec<DnsServer> = Vec::new();
        servers.append(
            &mut group
                .ip4
                .into_iter()
                .map(DnsServer::new_dns)
                .collect::<Result<Vec<_>, _>>()?,
        );
        servers.append(
            &mut group
                .ip6
                .into_iter()
                .map(DnsServer::new_dns)
                .collect::<Result<Vec<_>, _>>()?,
        );
        servers.append(
            &mut futures::future::try_join_all(
                group
                    .dot
                    .into_iter()
                    .map(|server| DnsServer::new_dot(server, &system_resolver)),
            )
            .await?,
        );
        let servers = servers;

        let count = servers.len();

        if count == 0 {
            continue;
        };

        let chunk_size = min(1000, domains.len() / count);
        println!(
            "benchmarking {} servers -> {}/chunk: {}",
            count, chunk_size, group.name
        );

        let mut domain_chunks = domains.chunks(chunk_size);

        for dns_server in servers {
            let domain_chunk: Vec<String> = domain_chunks.next().ok_or("Not enough domains")?.into();

            results
                .entry(dns_server)
                .insert_entry((Vec::<Duration>::new(), domain_chunk));
        }
    }

    println!("Running benchmark...");

    let multi_progress = Arc::new(MultiProgress::new());

    let server_count = results.len();
    let server_progress = Arc::new(multi_progress.add(ProgressBar::new(server_count as u64)));
    server_progress.set_style(
        ProgressStyle::default_bar()
            .template("  [{bar:40.green/blue}] {pos}/{len} servers completed")?
            .progress_chars("#>-"),
    );
    server_progress.tick();

    let total_domains: usize = results.values().map(|(_, domains)| domains.len()).sum();
    let domain_progress = Arc::new(multi_progress.add(ProgressBar::new(total_domains as u64)));
    domain_progress.set_style(
        ProgressStyle::with_template("{spinner:.green} [{bar:40.green/blue}] {elapsed_precise}, {per_sec:1} - {pos}/{len} domains queried")?
            .progress_chars("#>-")
            .tick_chars("⡏⠟⠻⢹⣸⣴⣦⣇ "),
    );
    domain_progress.enable_steady_tick(Duration::from_millis(300));

    let server_futures =
        results
            .iter_mut()
            .map(|(server, (durations, server_domains))| {
                let domain_progress = Arc::clone(&domain_progress);

                async move {
                    let mut rng = rand::rng();
                    tokio::time::sleep(Duration::from_millis(rng.random_range(0..5000))).await;

                    for domain in server_domains {
                        let (r, time) = measure_time_async(|| server.resolve4(domain)).await;
                        process_result(&server, durations, r, time, &domain_progress);

                        let (r, time) = measure_time_async(|| server.resolve6(domain)).await;
                        process_result(&server, durations, r, time, &domain_progress);

                        tokio::time::sleep(Duration::from_millis(rng.random_range(10..30))).await;

                        domain_progress.inc(1);
                    }
                }
            });

    let mut stream = FuturesUnordered::from_iter(server_futures);
    while stream.next().await.is_some() {
        server_progress.inc(1);
    }
    drop(stream);

    server_progress.finish_with_message("All servers completed");
    drop(multi_progress);
    println!();

    let results = results
        .into_iter()
        .map(|(server, (durations, _))| (server, durations))
        .collect::<IndexMap<_, _>>();

    println!("save results");

    std::fs::create_dir_all("results")?;
    let filename = format!("results/dns-benchmark-{timestamp}.csv");
    let mut csv = csv::WriterBuilder::default()
        .quote_style(QuoteStyle::NonNumeric)
        .from_path(&filename)?;

    csv.write_record(results.keys().map(ToString::to_string))
        .map_err(|err| format!("Failed to write csv header: {err}"))?;

    let max_len = results.values().map(|v| v.len()).max().unwrap_or(0);
    for i in 0..max_len {
        let mut record = Vec::<Cow<str>>::new();
        for durations in results.values() {
            if let Some(duration) = durations.get(i) {
                record.push(duration.as_millis().to_string().into());
            } else {
                record.push("".into());
            }
        }
        csv.write_record(record.iter().map(|s| s.as_ref()))
            .map_err(|err| format!("Failed to write csv record: {err}"))?;
    }

    println!("Results saved to {}", filename);
    Ok(())
}

fn process_result<S: Display, R>(
    server: &S,
    result: &mut Vec<Duration>,
    r: Result<R, ResolveError>,
    time: Duration,
    domain_progress: &ProgressBar,
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
            domain_progress.suspend(|| {
                eprintln!("{}: {:?}", server, e);
            });
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
    let duration = end.duration_since(start).expect("time should go forward");
    (r, duration)
}

fn timestamp(st: SystemTime) -> String {
    let dt: DateTime<Utc> = st.into();
    dt.format("%Y-%m-%d_%H:%M:%S").to_string()
}
