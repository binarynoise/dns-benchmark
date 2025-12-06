use std::borrow::Cow;
use std::error::Error;
use std::time::Duration;
use csv::{QuoteStyle, WriterBuilder};
use indexmap::IndexMap;
use crate::dns_server::DnsServer;

pub fn save_results_to_csv(
    results: &IndexMap<DnsServer, Vec<Duration>>,
    timestamp: &str,
) -> Result<String, Box<dyn Error>> {
    println!("Saving results to csv");
    std::fs::create_dir_all("results")?;
    let filename = format!("results/dns-benchmark-{timestamp}.csv");
    let mut csv = WriterBuilder::default()
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
    Ok(filename)
}
