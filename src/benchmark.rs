use crate::config::{AppConfig, GroupConfig};
use crate::dns_server::DnsServer;
use futures::stream::{FuturesUnordered, StreamExt};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::ProtoErrorKind;
use hickory_resolver::{ResolveError, ResolveErrorKind, Resolver};
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::Rng;
use std::cmp::min;
use std::error::Error;
use std::fmt::Display;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub struct BenchmarkResults {
    pub results: IndexMap<DnsServer, Vec<Duration>>,
}

pub async fn run_benchmark(
    config: &AppConfig,
    domains: Vec<String>,
) -> Result<BenchmarkResults, Box<dyn Error>> {
    let system_resolver = Resolver::builder(TokioConnectionProvider::default())?.build();

    let mut results: IndexMap<DnsServer, (Vec<Duration>, Vec<String>)> = IndexMap::new();

    for group in &config.groups {
        let servers = setup_servers(group, &system_resolver).await;

        if servers.is_empty() {
            continue;
        }

        let count = servers.len();
        let chunk_size = min(1000, domains.len() / count);

        println!("benchmarking {} servers -> {}/chunk: {}", count, chunk_size, group.name);

        distribute_domains_to_servers(servers, &domains, chunk_size, &mut results);
    }

    execute_benchmark(&mut results).await;

    Ok(BenchmarkResults {
        results: results
            .into_iter()
            .map(|(server, (durations, _))| (server, durations))
            .collect::<IndexMap<_, _>>(),
    })
}

async fn setup_servers(
    group: &GroupConfig,
    system_resolver: &Resolver<TokioConnectionProvider>,
) -> Vec<DnsServer> {
    let mut servers: Vec<DnsServer> = Vec::new();

    servers.append(
        &mut group
            .ip4
            .clone()
            .into_iter()
            .map(DnsServer::new_dns)
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to create IPv4 DNS servers"),
    );

    servers.append(
        &mut group
            .ip6
            .clone()
            .into_iter()
            .map(DnsServer::new_dns)
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to create IPv6 DNS servers"),
    );

    servers.append(
        &mut futures::future::try_join_all(
            group
                .dot
                .clone()
                .into_iter()
                .map(|server| DnsServer::new_dot(server, system_resolver)),
        )
        .await
        .expect("Failed to create DoT servers"),
    );

    servers
}

fn distribute_domains_to_servers(
    servers: Vec<DnsServer>,
    domains: &[String],
    chunk_size: usize,
    results: &mut IndexMap<DnsServer, (Vec<Duration>, Vec<String>)>,
) {
    let mut domain_chunks = domains.chunks(chunk_size);

    for dns_server in servers {
        let domain_chunk: Vec<String> = domain_chunks.next().expect("Not enough domains").into();

        results
            .entry(dns_server)
            .insert_entry((Vec::<Duration>::new(), domain_chunk));
    }
}

async fn execute_benchmark(results: &mut IndexMap<DnsServer, (Vec<Duration>, Vec<String>)>) {
    println!("Running benchmark...");

    let multi_progress = Arc::new(MultiProgress::new());

    let server_count = results.len();
    let server_progress = Arc::new(multi_progress.add(ProgressBar::new(server_count as u64)));
    server_progress.set_style(
        ProgressStyle::default_bar()
            .template("  [{bar:40.green/blue}] {pos}/{len} servers completed")
            .expect("Failed to set server progress style")
            .progress_chars("#>-"),
    );
    server_progress.tick();

    let total_domains: usize = results.values().map(|(_, domains)| domains.len()).sum();
    let domain_progress = Arc::new(multi_progress.add(ProgressBar::new(total_domains as u64)));
    domain_progress.set_style(
        ProgressStyle::with_template("{spinner:.green} [{bar:40.green/blue}] {elapsed_precise}, {per_sec:1} - {pos}/{len} domains queried")
            .expect("Failed to set domain progress style")
            .progress_chars("#>-")
    );
    domain_progress.enable_steady_tick(Duration::from_millis(300));

    let server_futures = results
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
            }
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
