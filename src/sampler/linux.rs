use super::kernel_interface::KernelInterface;
use super::{SampleError, Sampler};
use crate::model::{BootInfo, CpuTicks, LoadAverage, MemoryStats, RawProcessInfo, RawSample};

pub struct LinuxSampler {
    kernel: Box<dyn KernelInterface>,
}

impl LinuxSampler {
    pub fn new() -> Self {
        LinuxSampler {
            kernel: Box::new(LinuxKernel),
        }
    }
}

impl Sampler for LinuxSampler {
    fn sample(&mut self) -> Result<RawSample, SampleError> {
        self.kernel
            .get_processor_info()
            .map_err(SampleError::System)
    }
}

pub struct LinuxKernel;

impl KernelInterface for LinuxKernel {
    fn get_processor_info(&self) -> Result<RawSample, i32> {
        let cpu_ticks = self.parse_cpu_stat()?;
        let cpu_count = cpu_ticks.len();
        let boot_info = self.get_boot_info()?;
        let load_average = self.get_load_average()?;
        let memory_stats = self.get_memory_stats()?;
        let processes = self.get_processes().unwrap_or_default();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(RawSample {
            timestamp,
            cpu_count,
            cpu_ticks,
            boot_info,
            load_average,
            memory_stats,
            processes,
        })
    }

    fn get_boot_info(&self) -> Result<BootInfo, i32> {
        Ok(BootInfo { boot_time_secs: 0 })
    }

    fn get_load_average(&self) -> Result<LoadAverage, i32> {
        Ok(LoadAverage {
            one_min: 0.0,
            five_min: 0.0,
            fifteen_min: 0.0,
        })
    }

    fn get_memory_stats(&self) -> Result<MemoryStats, i32> {
        Ok(MemoryStats {
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
        })
    }

    fn get_processes(&self) -> Result<Vec<RawProcessInfo>, i32> {
        Ok(vec![])
    }
}

impl LinuxKernel {
    fn parse_cpu_stat(&self) -> Result<Vec<CpuTicks>, i32> {
        Ok(vec![])
    }
}
