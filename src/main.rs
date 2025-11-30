mod model;
mod sampler;

use model::Snapshot;
use sampler::{PlatformSampler as Sampler, Sampler as _};
use std::thread;
use std::time::Duration;

const SAMPLE_INTERVAL_SECS: u64 = 2;

fn main() {
    let mut sampler = Sampler::new();
    let mut previous_sample = None;

    println!("Starting monitoring loop...\n");

    loop {
        let current_sample = match sampler.sample() {
            Ok(sample) => sample,
            Err(e) => {
                eprintln!("Error sampling: {:?}", e);
                thread::sleep(Duration::from_secs(SAMPLE_INTERVAL_SECS));
                continue;
            }
        };

        if let Some(prev) = previous_sample {
            let snapshot = Snapshot::compute(&current_sample, &prev);
            snapshot.render();
        }

        previous_sample = Some(current_sample);
        thread::sleep(Duration::from_secs(SAMPLE_INTERVAL_SECS));
    }
}
