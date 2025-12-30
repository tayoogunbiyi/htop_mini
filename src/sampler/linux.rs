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

fn parse_cpu_stat_content(content: &str) -> Result<Vec<CpuTicks>, i32> {
    let mut cpu_ticks = Vec::new();

    for line in content.lines() {
        // Skip aggregate "cpu" line, only process "cpu0", "cpu1", etc.
        if line.starts_with("cpu") && !line.starts_with("cpu ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Format: cpuN user nice system idle iowait irq softirq [steal [guest [guest_nice]]]
            if parts.len() >= 5 {
                let user = parts[1].parse::<u32>().unwrap_or(0);
                let nice = parts[2].parse::<u32>().unwrap_or(0);
                let system = parts[3].parse::<u32>().unwrap_or(0);
                let idle = parts[4].parse::<u32>().unwrap_or(0);
                let iowait = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0u32);
                let irq = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0u32);
                let softirq = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0u32);
                let steal = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0u32);

                cpu_ticks.push(CpuTicks {
                    user,
                    nice,
                    system: system.saturating_add(irq).saturating_add(softirq),
                    idle: idle.saturating_add(iowait).saturating_add(steal),
                });
            }
        }
    }

    if cpu_ticks.is_empty() {
        return Err(-1);
    }

    Ok(cpu_ticks)
}

impl LinuxKernel {
    fn parse_cpu_stat(&self) -> Result<Vec<CpuTicks>, i32> {
        let content = std::fs::read_to_string("/proc/stat").map_err(|_| -1)?;
        parse_cpu_stat_content(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_stat_content() {
        let sample_stat = r#"cpu  12345 678 9012 345678 901 234 567 0 0 0
cpu0 6172 339 4506 172839 450 117 283 0 0 0
cpu1 6173 339 4506 172839 451 117 284 0 0 0
intr 12345678 0 0 0 0
ctxt 98765432
btime 1703123456
processes 12345
procs_running 2
procs_blocked 0
"#;

        let cpu_ticks = parse_cpu_stat_content(sample_stat).unwrap();

        assert_eq!(cpu_ticks.len(), 2);
        assert_eq!(cpu_ticks[0].user, 6172);
        assert_eq!(cpu_ticks[0].nice, 339);
        assert_eq!(cpu_ticks[0].system, 4506 + 117 + 283);
        assert_eq!(cpu_ticks[0].idle, 172839 + 450 + 0);
    }

    #[test]
    fn test_parse_cpu_stat_content_empty() {
        let result = parse_cpu_stat_content("");
        assert_eq!(result, Err(-1));
    }

    #[test]
    fn test_parse_cpu_stat_content_no_per_cpu_lines() {
        let sample = "cpu  12345 678 9012 345678 901 234 567 0 0 0\n";
        let result = parse_cpu_stat_content(sample);
        assert_eq!(result, Err(-1));
    }
}
