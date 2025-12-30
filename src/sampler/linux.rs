use super::kernel_interface::KernelInterface;
use super::{SampleError, Sampler};
use crate::model::{
    BootInfo, CpuTicks, LoadAverage, MemoryStats, ProcessState, RawProcessInfo, RawSample,
};

use libc::{c_char, getpwuid, uid_t};

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
            .as_nanos();

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
        Self::parse_boot_time(&content)
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
        Self::parse_meminfo(&content, page_size)
    }

    fn get_processes(&self) -> Result<Vec<RawProcessInfo>, i32> {
        let mut processes = Vec::new();
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
        let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 };

        let proc_dir = std::fs::read_dir("/proc").map_err(|_| -1)?;

        for entry in proc_dir.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if let Ok(pid) = name_str.parse::<i32>() {
                if let Ok(proc_info) = Self::parse_process(pid, page_size, clk_tck) {
                    processes.push(proc_info);
                }
            }
        }

        Ok(processes)
    }
}

impl LinuxKernel {
    fn parse_cpu_stat(&self) -> Result<Vec<CpuTicks>, i32> {
        let content = std::fs::read_to_string("/proc/stat").map_err(|_| -1)?;
        Self::parse_cpu_ticks(&content)
    }

    fn parse_cpu_ticks(content: &str) -> Result<Vec<CpuTicks>, i32> {
        let mut cpu_ticks = Vec::new();

        for line in content.lines() {
            // Skip aggregate "cpu" line, only process "cpu0", "cpu1", etc.
            if line.starts_with("cpu") && !line.starts_with("cpu ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // Format: cpuN user nice system idle iowait irq softirq [steal [guest [guest_nice]]]
                if parts.len() >= 5 {
                    let user = parts[1].parse::<u64>().unwrap_or(0);
                    let nice = parts[2].parse::<u64>().unwrap_or(0);
                    let system = parts[3].parse::<u64>().unwrap_or(0);
                    let idle = parts[4].parse::<u64>().unwrap_or(0);
                    let iowait = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0u64);
                    let irq = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0u64);
                    let softirq = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0u64);
                    let steal = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0u64);

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

    fn parse_boot_time(content: &str) -> Result<BootInfo, i32> {
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

    fn parse_meminfo(content: &str, page_size: u64) -> Result<MemoryStats, i32> {
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
            compressed_bytes: 0,
            free_bytes: mem_free_kb * 1024,
            purgeable_bytes: buffers_kb * 1024,
            page_size,
            swap_total_bytes: swap_total_kb * 1024,
            swap_used_bytes: (swap_total_kb.saturating_sub(swap_free_kb)) * 1024,
        })
    }

    fn parse_process(pid: i32, page_size: u64, clk_tck: u64) -> Result<RawProcessInfo, ()> {
        let stat_path = format!("/proc/{}/stat", pid);
        let stat_content = std::fs::read_to_string(&stat_path).map_err(|_| ())?;

        let (comm, fields) = Self::parse_stat_line(&stat_content)?;

        // Fields after comm (0-indexed): state(0), ppid(1), pgrp(2), session(3), tty_nr(4),
        // tpgid(5), flags(6), minflt(7), cminflt(8), majflt(9), cmajflt(10),
        // utime(11), stime(12), cutime(13), cstime(14), priority(15), nice(16),
        // num_threads(17), itrealvalue(18), starttime(19), vsize(20), rss(21), ...

        let state = match fields.first().and_then(|s| s.chars().next()).unwrap_or('?') {
            'R' => ProcessState::Running,
            'S' | 'D' | 'I' => ProcessState::Sleeping,
            'T' | 't' => ProcessState::Stopped,
            'Z' => ProcessState::Zombie,
            _ => ProcessState::Unknown,
        };

        let utime: u64 = fields.get(11).and_then(|s| s.parse().ok()).unwrap_or(0);
        let stime: u64 = fields.get(12).and_then(|s| s.parse().ok()).unwrap_or(0);
        let priority: i32 = fields.get(15).and_then(|s| s.parse().ok()).unwrap_or(0);
        let nice: i32 = fields.get(16).and_then(|s| s.parse().ok()).unwrap_or(0);
        let num_threads: u32 = fields.get(17).and_then(|s| s.parse().ok()).unwrap_or(1);
        let vsize: u64 = fields.get(20).and_then(|s| s.parse().ok()).unwrap_or(0);
        let rss_pages: u64 = fields.get(21).and_then(|s| s.parse().ok()).unwrap_or(0);

        let rss_bytes = rss_pages * page_size;
        let cpu_time_ns = if clk_tck > 0 {
            ((utime + stime) * 1_000_000_000) / clk_tck
        } else {
            0
        };

        let running_threads = Self::count_running_threads(pid);
        let uid = Self::get_process_uid(pid);
        let user = Self::get_username(uid);
        let command = Self::get_cmdline(pid).unwrap_or_else(|| comm.clone());

        Ok(RawProcessInfo {
            pid,
            uid,
            user,
            priority,
            nice,
            virtual_size: vsize,
            resident_size: rss_bytes,
            state,
            cpu_time_ns,
            thread_count: num_threads,
            running_threads,
            command,
        })
    }

    fn parse_stat_line(content: &str) -> Result<(String, Vec<&str>), ()> {
        let start = content.find('(').ok_or(())?;
        let end = content.rfind(')').ok_or(())?;

        if end <= start {
            return Err(());
        }

        let comm = content[start + 1..end].to_string();
        let rest = content.get(end + 2..).ok_or(())?;
        let fields: Vec<&str> = rest.split_whitespace().collect();

        Ok((comm, fields))
    }

    fn count_running_threads(pid: i32) -> u32 {
        let task_dir = format!("/proc/{}/task", pid);
        let mut running = 0;

        if let Ok(entries) = std::fs::read_dir(&task_dir) {
            for entry in entries.flatten() {
                let tid = entry.file_name();
                let stat_path = format!("{}/{}/stat", task_dir, tid.to_string_lossy());
                if let Ok(content) = std::fs::read_to_string(&stat_path) {
                    if let Some(end_paren) = content.rfind(')') {
                        let after_comm = content.get(end_paren + 2..);
                        if let Some(rest) = after_comm {
                            if rest.starts_with('R') {
                                running += 1;
                            }
                        }
                    }
                }
            }
        }

        running
    }

    fn get_process_uid(pid: i32) -> u32 {
        let status_path = format!("/proc/{}/status", pid);
        if let Ok(content) = std::fs::read_to_string(&status_path) {
            for line in content.lines() {
                if line.starts_with("Uid:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        return parts[1].parse().unwrap_or(0);
                    }
                }
            }
        }
        0
    }

    fn get_username(uid: u32) -> String {
        unsafe {
            let pw = getpwuid(uid as uid_t);
            if !pw.is_null() && !(*pw).pw_name.is_null() {
                std::ffi::CStr::from_ptr((*pw).pw_name as *const c_char)
                    .to_string_lossy()
                    .to_string()
            } else {
                uid.to_string()
            }
        }
    }

    fn get_cmdline(pid: i32) -> Option<String> {
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        let content = std::fs::read(&cmdline_path).ok()?;

        if content.is_empty() {
            return None;
        }

        let cmdline: String = content
            .into_iter()
            .map(|b| if b == 0 { ' ' } else { b as char })
            .collect();

        Some(cmdline.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_ticks() {
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

        let cpu_ticks = LinuxKernel::parse_cpu_ticks(sample_stat).unwrap();

        assert_eq!(cpu_ticks.len(), 2);
        assert_eq!(cpu_ticks[0].user, 6172);
        assert_eq!(cpu_ticks[0].nice, 339);
        assert_eq!(cpu_ticks[0].system, 4506 + 117 + 283);
        assert_eq!(cpu_ticks[0].idle, 172839 + 450 + 0);
    }

    #[test]
    fn test_parse_cpu_ticks_empty() {
        let result = LinuxKernel::parse_cpu_ticks("");
        assert_eq!(result, Err(-1));
    }

    #[test]
    fn test_parse_cpu_ticks_no_per_cpu_lines() {
        let sample = "cpu  12345 678 9012 345678 901 234 567 0 0 0\n";
        let result = LinuxKernel::parse_cpu_ticks(sample);
        assert_eq!(result, Err(-1));
    }

    #[test]
    fn test_parse_boot_time() {
        let sample = "cpu  12345 678 9012\nbtime 1703123456\nprocesses 12345\n";
        let result = LinuxKernel::parse_boot_time(sample).unwrap();
        assert_eq!(result.boot_time_secs, 1703123456);
    }

    #[test]
    fn test_parse_boot_time_missing() {
        let sample = "cpu  12345 678 9012\nprocesses 12345\n";
        let result = LinuxKernel::parse_boot_time(sample);
        assert_eq!(result, Err(-1));
    }

    #[test]
    fn test_parse_meminfo() {
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

        let result = LinuxKernel::parse_meminfo(sample, 4096).unwrap();

        assert_eq!(result.total_memory_bytes, 16384000 * 1024);
        assert_eq!(result.free_bytes, 8192000 * 1024);
        assert_eq!(result.active_bytes, 4000000 * 1024);
        assert_eq!(result.inactive_bytes, 2000000 * 1024);
        assert_eq!(result.wired_bytes, 500000 * 1024);
        assert_eq!(result.purgeable_bytes, 512000 * 1024);
        assert_eq!(result.swap_total_bytes, 8000000 * 1024);
        assert_eq!(result.swap_used_bytes, 500000 * 1024);
        assert_eq!(result.page_size, 4096);
    }

    #[test]
    fn test_parse_meminfo_empty() {
        let result = LinuxKernel::parse_meminfo("", 4096);
        assert_eq!(result, Err(-1));
    }

    #[test]
    fn test_parse_stat_line_simple() {
        let stat = "1234 (bash) S 1233 1234 1234 0 -1 4194304 1000 0 0 0 100 50 0 0 20 0 1 0 12345 67890 1234 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0";
        let (comm, fields) = LinuxKernel::parse_stat_line(stat).unwrap();
        assert_eq!(comm, "bash");
        assert_eq!(fields[0], "S");
        assert_eq!(fields[11], "100");
        assert_eq!(fields[12], "50");
        assert_eq!(fields[15], "20");
        assert_eq!(fields[16], "0");
        assert_eq!(fields[17], "1");
    }

    #[test]
    fn test_parse_stat_line_with_spaces_in_comm() {
        let stat = "5678 (Web Content) S 5677 5678 5678 0 -1 4194304 2000 0 0 0 200 100 0 0 20 0 5 0 54321 98765 4321 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0";
        let (comm, fields) = LinuxKernel::parse_stat_line(stat).unwrap();
        assert_eq!(comm, "Web Content");
        assert_eq!(fields[0], "S");
        assert_eq!(fields[17], "5");
    }

    #[test]
    fn test_parse_stat_line_with_parens_in_comm() {
        let stat = "9999 ((sd-pam)) S 1 9999 9999 0 -1 1077936192 100 0 0 0 10 5 0 0 20 0 1 0 98765 12345 500 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0";
        let (comm, fields) = LinuxKernel::parse_stat_line(stat).unwrap();
        assert_eq!(comm, "(sd-pam)");
        assert_eq!(fields[0], "S");
    }
}
