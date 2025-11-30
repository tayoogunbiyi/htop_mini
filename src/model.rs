#[derive(Debug, Clone)]
pub struct RawSample {
    pub cpu_count: usize,
    pub cpu_ticks: Vec<CpuTicks>,
}

#[derive(Debug, Clone)]
pub struct CpuTicks {
    pub user: u32,
    pub system: u32,
    pub idle: u32,
    pub nice: u32,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub cpu_count: usize,
    pub cpu_usage: Vec<CpuUsage>,
}

#[derive(Debug, Clone)]
pub struct CpuUsage {
    pub user_percent: f64,
    pub system_percent: f64,
    pub idle_percent: f64,
    pub nice_percent: f64,
}

impl Snapshot {
    pub fn compute(current: &RawSample, previous: &RawSample) -> Self {
        assert_eq!(current.cpu_count, previous.cpu_count, "CPU count mismatch");

        let cpu_usage: Vec<CpuUsage> = current.cpu_ticks.iter()
            .zip(previous.cpu_ticks.iter())
            .map(|(curr, prev)| {
                let delta_user = curr.user.wrapping_sub(prev.user);
                let delta_system = curr.system.wrapping_sub(prev.system);
                let delta_idle = curr.idle.wrapping_sub(prev.idle);
                let delta_nice = curr.nice.wrapping_sub(prev.nice);

                let total_delta = delta_user + delta_system + delta_idle + delta_nice;

                if total_delta > 0 {
                    CpuUsage {
                        user_percent: (delta_user as f64 / total_delta as f64) * 100.0,
                        system_percent: (delta_system as f64 / total_delta as f64) * 100.0,
                        idle_percent: (delta_idle as f64 / total_delta as f64) * 100.0,
                        nice_percent: (delta_nice as f64 / total_delta as f64) * 100.0,
                    }
                } else {
                    CpuUsage {
                        user_percent: 0.0,
                        system_percent: 0.0,
                        idle_percent: 0.0,
                        nice_percent: 0.0,
                    }
                }
            })
            .collect();

        Snapshot {
            cpu_count: current.cpu_count,
            cpu_usage,
        }
    }

    pub fn render(&self) {
        println!("\n=== CPU Usage Snapshot ===");
        for (i, cpu) in self.cpu_usage.iter().enumerate() {
            println!(
                "CPU {:2}: User={:5.2}%, System={:5.2}%, Idle={:5.2}%, Nice={:5.2}%",
                i,
                cpu.user_percent,
                cpu.system_percent,
                cpu.idle_percent,
                cpu.nice_percent
            );
        }
        println!("=========================\n");
    }
}