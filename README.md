# DNS Benchmark

## Description

Measure how quickly DNS servers respond and identify which DNS server is the fastest at your place.

## Usage

Before running the benchmark, you'll need to set up the configuration file.
Copy the `application.sample.conf` to `application.conf`,
then edit `config/application.conf` to specify your desired domains and DNS servers to test.

You can now build and run the benchmark:

```bash
cargo build --release
./target/release/dns-benchmark
```

## Results

The results will be saved to the `results/` directory.

You can import the results into dns-test.ods to get an analysis of the results: copy the contents of results/dns-benchmark-{timestamp}-{name}.csv to the "results" sheet in (a copy of) dns-test.ods by using the text import dialogue.
