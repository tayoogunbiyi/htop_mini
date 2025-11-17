mod model;
mod sampler;

use sampler::{PlatformSampler as Sampler, Sampler as _};
use std::thread;
use std::time::Duration;

const SAMPLE_INTERVAL_SECS: u64 = 2;
fn main() {
    let mut sampler = Sampler::new();

    loop {
        match sampler.sample() {
            Ok(raw_sample) => println!("Sample collected: {:?}", raw_sample),
            Err(e) => eprintln!("Error sampling: {:?}", e),
        }
        thread::sleep(Duration::from_secs(SAMPLE_INTERVAL_SECS));
    }
}
