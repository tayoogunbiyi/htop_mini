mod model;
mod sampler;

use model::Snapshot;
use sampler::{PlatformSampler as Sampler, Sampler as _};
use std::thread;
use std::time::Duration;

const SAMPLE_INTERVAL_SECS: u64 = 2;

fn main() {
    let mut sampler = Sampler::new();

    let mut previous_sample = match sampler.sample() {
        Ok(sample) => sample,
        Err(e) => {
            eprintln!("Error taking initial sample: {:?}", e);
            return;
        }
    };

    println!("Collected initial sample. Starting monitoring loop...\n");
    thread::sleep(Duration::from_secs(SAMPLE_INTERVAL_SECS));

    loop {
        let current_sample = match sampler.sample() {
            Ok(sample) => sample,
            Err(e) => {
                eprintln!("Error sampling: {:?}", e);
                thread::sleep(Duration::from_secs(SAMPLE_INTERVAL_SECS));
                continue;
            }
        };

        let snapshot = Snapshot::compute(&current_sample, &previous_sample);

        previous_sample = current_sample;

        snapshot.render();

        thread::sleep(Duration::from_secs(SAMPLE_INTERVAL_SECS));
    }
}
