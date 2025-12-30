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
        let content = std::fs::read_to_string("/proc/stat").map_err(|_| -1)?;
        parse_boot_time_content(&content)
    }

    fn get_load_average(&self) -> Result<LoadAverage, i32> {
        unsafe {
            let mut loadavg = [0.0f64; 3];
            let result = libc::getloadavg(loadavg.as_mut_ptr(), 3);

            if result != 3 {
                return Err(-1);
            }

            Ok(LoadAverage {
                one_min: loadavg[0],
                five_min: loadavg[1],
                fifteen_min: loadavg[2],
            })
        }
    }

    fn get_memory_stats(&self) -> Result<MemoryStats, i32> {
        let content = std::fs::read_to_string("/proc/meminfo").map_err(|_| -1)?;
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
        parse_meminfo_content(&content, page_size)
    }

    fn get_processes(&self) -> Result<Vec<RawProcessInfo>, i32> {
        Ok(vec![])
    }
}

fn parse_meminfo_content(content: &str, page_size: u64) -> Result<MemoryStats, i32> {
    let mut mem_total_kb = 0u64;
    let mut mem_free_kb = 0u64;
    let mut active_kb = 0u64;
    let mut inactive_kb = 0u64;
    let mut buffers_kb = 0u64;
    let mut slab_kb = 0u64;
    let mut swap_total_kb = 0u64;
    let mut swap_free_kb = 0u64;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let value = parts[1].parse::<u64>().unwrap_or(0);
            match parts[0] {
                "MemTotal:" => mem_total_kb = value,
                "MemFree:" => mem_free_kb = value,
                "Active:" => active_kb = value,
                "Inactive:" => inactive_kb = value,
                "Buffers:" => buffers_kb = value,
                "Slab:" => slab_kb = value,
                "SwapTotal:" => swap_total_kb = value,
                "SwapFree:" => swap_free_kb = value,
                _ => {}
            }
        }
    }

    if mem_total_kb == 0 {
        return Err(-1);
    }

    Ok(MemoryStats {
        total_memory_bytes: mem_total_kb * 1024,
        active_bytes: active_kb * 1024,
        inactive_bytes: inactive_kb * 1024,
        wired_bytes: slab_kb * 1024,
        compressed_bytes: 0, // Linux doesn't have direct equivalent
        free_bytes: mem_free_kb * 1024,
        purgeable_bytes: buffers_kb * 1024,
        page_size,
        swap_total_bytes: swap_total_kb * 1024,
        swap_used_bytes: (swap_total_kb.saturating_sub(swap_free_kb)) * 1024,
    })
}

fn parse_boot_time_content(content: &str) -> Result<BootInfo, i32> {
    for line in content.lines() {
        if line.starts_with("btime ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let boot_time = parts[1].parse::<u64>().map_err(|_| -1)?;
                return Ok(BootInfo { boot_time_secs: boot_time });
            }
        }
    }
    Err(-1)
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

    #[test]
    fn test_parse_boot_time_content() {
        let sample = "cpu  12345 678 9012\nbtime 1703123456\nprocesses 12345\n";
        let result = parse_boot_time_content(sample).unwrap();
        assert_eq!(result.boot_time_secs, 1703123456);
    }

    #[test]
    fn test_parse_boot_time_content_missing() {
        let sample = "cpu  12345 678 9012\nprocesses 12345\n";
        let result = parse_boot_time_content(sample);
        assert_eq!(result, Err(-1));
    }

    #[test]
    fn test_parse_meminfo_content() {
        let sample = r#"MemTotal:       16384000 kB
MemFree:         8192000 kB
MemAvailable:   12000000 kB
Buffers:          512000 kB
Cached:          3000000 kB
Active:          4000000 kB
Inactive:        2000000 kB
Slab:             500000 kB
SwapTotal:       8000000 kB
SwapFree:        7500000 kB
"#;

        let result = parse_meminfo_content(sample, 4096).unwrap();

        assert_eq!(result.total_memory_bytes, 16384000 * 1024);
        assert_eq!(result.free_bytes, 8192000 * 1024);
        assert_eq!(result.active_bytes, 4000000 * 1024);
        assert_eq!(result.inactive_bytes, 2000000 * 1024);
        assert_eq!(result.wired_bytes, 500000 * 1024); // Slab
        assert_eq!(result.purgeable_bytes, 512000 * 1024); // Buffers
        assert_eq!(result.swap_total_bytes, 8000000 * 1024);
        assert_eq!(result.swap_used_bytes, 500000 * 1024); // 8000000 - 7500000
        assert_eq!(result.page_size, 4096);
    }

    #[test]
    fn test_parse_meminfo_content_empty() {
        let result = parse_meminfo_content("", 4096);
        assert_eq!(result, Err(-1));
    }
}
