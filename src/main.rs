mod model;
mod sampler;

use sampler::{PlatformSampler as Sampler, Sampler as _};

fn main() {
    let mut sampler = Sampler::new();
    match sampler.sample() {
        Ok(raw_sample) => println!("Sample collected: {:?}", raw_sample),
        Err(e) => eprintln!("Error sampling: {:?}", e),
    }
}
