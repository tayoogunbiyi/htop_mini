use super::{Sampler, SampleError};
use crate::{RawSample};

pub struct LinuxSampler {
}

impl LinuxSampler {
    pub fn new() -> Self {
        LinuxSampler {}
    }
}

impl Sampler for LinuxSampler {
    fn sample(&mut self) -> Result<RawSample, SampleError> {
        Ok(RawSample { })
    }
}
