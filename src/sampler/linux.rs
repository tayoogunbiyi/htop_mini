use super::{Sampler, SampleError};
use crate::model::{BootInfo, LoadAverage, MemoryStats, RawSample};

pub struct LinuxSampler {}

impl LinuxSampler {
    pub fn new() -> Self {
        LinuxSampler {}
    }
}

impl Sampler for LinuxSampler {
    fn sample(&mut self) -> Result<RawSample, SampleError> {
        Ok(RawSample {
            cpu_count: 0,
            cpu_ticks: vec![],
            boot_info: BootInfo {
                boot_time_secs: 0,
            },
            load_average: LoadAverage {
                one_min: 0.0,
                five_min: 0.0,
                fifteen_min: 0.0,
            },
            memory_stats: MemoryStats {
                total_memory_bytes: 0,
                active_bytes: 0,
                inactive_bytes: 0,
                wired_bytes: 0,
                compressed_bytes: 0,
                free_bytes: 0,
                purgeable_bytes: 0,
                page_size: 0,
                swap_total_bytes: 0,
                swap_used_bytes: 0,
            },
            processes: vec![],
        })
    }
}
