use std::error::Error;
use crate::config::DomainConfig;
use rand::seq::SliceRandom;
use sprintf::sprintf;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn load_domains(config: &DomainConfig) -> Result<Vec<String>, Box<dyn Error>> {
    let mut domains: Vec<String> = Vec::new();

    if let Some(fixed) = &config.fixed {
        for _ in 0..config.repeat {
            domains.push(fixed.clone());
        }
    }

    if let Some(format) = &config.format {
        let prefix: u32 = rand::random();
        for i in 0..config.repeat {
            domains.push(sprintf!(format.as_str(), prefix, i)?);
        }
    }

    if let Some(file) = &config.file {
        let file = File::open(file).map_err(|err| format!("Could not open file {file}: {err}"))?;
        let reader = BufReader::new(file);
        let mut lines: Vec<String> = reader.lines().skip(1).collect::<Result<_, _>>()?;
        lines.shuffle(&mut rand::rng());
        domains.extend(lines);
    }

    for domain in &mut domains {
        if !domain.ends_with(".") {
            domain.push('.');
        }
    }

    println!("Loaded {} domains", domains.len());
    Ok(domains)
}
