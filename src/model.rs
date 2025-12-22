#[derive(Debug, Clone)]
pub struct RawSample {
    pub timestamp: u64,
    pub cpu_count: usize,
    pub cpu_ticks: Vec<CpuTicks>,
    pub boot_info: BootInfo,
    pub load_average: LoadAverage,
    pub memory_stats: MemoryStats,
    pub processes: Vec<RawProcessInfo>,
}

impl RawSample {
    pub fn task_stats(&self) -> TaskStats {
        TaskStats {
            total_tasks: self.processes.len() as u32,
            total_threads: self.processes.iter().map(|p| p.thread_count).sum(),
            running_threads: self.processes.iter().map(|p| p.running_threads).sum(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskStats {
    pub total_tasks: u32,
    pub total_threads: u32,
    pub running_threads: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Unknown,
}

impl ProcessState {
    pub fn as_char(self) -> char {
        match self {
            ProcessState::Running => 'R',
            ProcessState::Sleeping => 'S',
            ProcessState::Stopped => 'T',
            ProcessState::Zombie => 'Z',
            ProcessState::Unknown => '?',
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawProcessInfo {
    pub pid: i32,
    pub uid: u32,
    pub user: String,
    pub priority: i32,
    pub nice: i32,
    pub virtual_size: u64,
    pub resident_size: u64,
    pub state: ProcessState,
    pub cpu_time_ns: u64,
    pub thread_count: u32,
    pub running_threads: u32,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: i32,
    pub user: String,
    pub priority: i32,
    pub nice: i32,
    pub virtual_size: u64,
    pub resident_size: u64,
    pub state: ProcessState,
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub cpu_time_secs: f64,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct CpuTicks {
    pub user: u32,
    pub system: u32,
    pub idle: u32,
    pub nice: u32,
}

#[derive(Debug, Clone)]
pub struct BootInfo {
    pub boot_time_secs: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_memory_bytes: u64,
    pub active_bytes: u64,
    pub inactive_bytes: u64,
    pub wired_bytes: u64,
    pub compressed_bytes: u64,
    pub free_bytes: u64,
    pub purgeable_bytes: u64,
    pub page_size: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

const BYTES_PER_GB: f64 = 1_073_741_824.0;

impl MemoryStats {
    pub fn used_bytes(&self) -> u64 {
        self.active_bytes + self.wired_bytes + self.compressed_bytes
    }

    pub fn total_gb(&self) -> f64 {
        self.total_memory_bytes as f64 / BYTES_PER_GB
    }

    pub fn used_gb(&self) -> f64 {
        self.used_bytes() as f64 / BYTES_PER_GB
    }

    pub fn used_percent(&self) -> f64 {
        (self.used_bytes() as f64 / self.total_memory_bytes as f64) * 100.0
    }

    pub fn active_gb(&self) -> f64 {
        self.active_bytes as f64 / BYTES_PER_GB
    }

    pub fn wired_gb(&self) -> f64 {
        self.wired_bytes as f64 / BYTES_PER_GB
    }

    pub fn compressed_gb(&self) -> f64 {
        self.compressed_bytes as f64 / BYTES_PER_GB
    }

    pub fn swap_total_gb(&self) -> f64 {
        self.swap_total_bytes as f64 / BYTES_PER_GB
    }

    pub fn swap_used_gb(&self) -> f64 {
        self.swap_used_bytes as f64 / BYTES_PER_GB
    }

    pub fn swap_used_percent(&self) -> f64 {
        if self.swap_total_bytes > 0 {
            (self.swap_used_bytes as f64 / self.swap_total_bytes as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub cpu_count: usize,
    pub cpu_usage: Vec<CpuUsage>,
    pub uptime: Uptime,
    pub load_average: LoadAverage,
    pub memory_stats: MemoryStats,
    pub task_stats: TaskStats,
    pub processes: Vec<ProcessInfo>,
}

#[derive(Debug, Clone)]
pub struct Uptime {
    total_seconds: u64,
    pub days: u64,
    pub hours: u64,
    pub minutes: u64,
    pub seconds: u64,
}

impl Uptime {
    pub fn from_seconds(total_seconds: u64) -> Self {
        Self {
            total_seconds,
            days: total_seconds / 86400,
            hours: (total_seconds % 86400) / 3600,
            minutes: (total_seconds % 3600) / 60,
            seconds: total_seconds % 60,
        }
    }

    pub fn total_seconds(&self) -> u64 {
        self.total_seconds
    }
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

        let cpu_usage: Vec<CpuUsage> = current
            .cpu_ticks
            .iter()
            .zip(previous.cpu_ticks.iter())
            .map(|(curr, prev)| {
                let delta_user = curr.user.wrapping_sub(prev.user);
                let delta_system = curr.system.wrapping_sub(prev.system);
                let delta_idle = curr.idle.wrapping_sub(prev.idle);
                let delta_nice = curr.nice.wrapping_sub(prev.nice);

                let total_delta =
                    delta_user as u64 + delta_system as u64 + delta_idle as u64 + delta_nice as u64;

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

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let uptime_secs = now - current.boot_info.boot_time_secs;

        let uptime = Uptime::from_seconds(uptime_secs);

        let prev_processes: std::collections::HashMap<i32, &RawProcessInfo> = previous
            .processes
            .iter()
            .map(|p| (p.pid, p))
            .collect();

        let total_memory = current.memory_stats.total_memory_bytes;

        let sample_interval_secs = current.timestamp.saturating_sub(previous.timestamp);
        let sample_interval_ns = sample_interval_secs * 1_000_000_000;

        let mut processes: Vec<ProcessInfo> = current
            .processes
            .iter()
            .map(|curr_proc| {
                let cpu_percent = if let Some(prev_proc) = prev_processes.get(&curr_proc.pid) {
                    let delta_ns = curr_proc.cpu_time_ns.saturating_sub(prev_proc.cpu_time_ns);
                    if sample_interval_ns > 0 {
                        (delta_ns as f64 / sample_interval_ns as f64) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                let mem_percent = if total_memory > 0 {
                    (curr_proc.resident_size as f64 / total_memory as f64) * 100.0
                } else {
                    0.0
                };

                ProcessInfo {
                    pid: curr_proc.pid,
                    user: curr_proc.user.clone(),
                    priority: curr_proc.priority,
                    nice: curr_proc.nice,
                    virtual_size: curr_proc.virtual_size,
                    resident_size: curr_proc.resident_size,
                    state: curr_proc.state,
                    cpu_percent,
                    mem_percent,
                    cpu_time_secs: curr_proc.cpu_time_ns as f64 / 1_000_000_000.0,
                    command: curr_proc.command.clone(),
                }
            })
            .collect();

        processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());

        Snapshot {
            cpu_count: current.cpu_count,
            cpu_usage,
            uptime,
            load_average: current.load_average,
            memory_stats: current.memory_stats,
            task_stats: current.task_stats(),
            processes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn make_memory_stats() -> MemoryStats {
        MemoryStats {
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            active_bytes: 4 * 1024 * 1024 * 1024,
            inactive_bytes: 2 * 1024 * 1024 * 1024,
            wired_bytes: 1 * 1024 * 1024 * 1024,
            compressed_bytes: 512 * 1024 * 1024,
            free_bytes: 8 * 1024 * 1024 * 1024,
            purgeable_bytes: 100 * 1024 * 1024,
            page_size: 16384,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
        }
    }

    fn make_single_cpu_sample(user: u32, system: u32, idle: u32, nice: u32) -> RawSample {
        RawSample {
            timestamp: 1000000,
            cpu_count: 1,
            cpu_ticks: vec![CpuTicks {
                user,
                system,
                idle,
                nice,
            }],
            boot_info: BootInfo {
                boot_time_secs: 1000000,
            },
            load_average: LoadAverage {
                one_min: 1.0,
                five_min: 1.5,
                fifteen_min: 2.0,
            },
            memory_stats: make_memory_stats(),
            processes: vec![],
        }
    }

    fn make_sample(cpu_count: usize, ticks: Vec<[u32; 4]>) -> RawSample {
        RawSample {
            timestamp: 1000000,
            cpu_count,
            cpu_ticks: ticks
                .iter()
                .map(|[u, s, i, n]| CpuTicks {
                    user: *u,
                    system: *s,
                    idle: *i,
                    nice: *n,
                })
                .collect(),
            boot_info: BootInfo {
                boot_time_secs: 1000000,
            },
            load_average: LoadAverage {
                one_min: 1.0,
                five_min: 1.5,
                fifteen_min: 2.0,
            },
            memory_stats: make_memory_stats(),
            processes: vec![],
        }
    }

    fn make_wraparound_sample() -> (RawSample, RawSample) {
        let mut previous = make_single_cpu_sample(u32::MAX - 50, 100, 100, 0);
        let mut current = make_single_cpu_sample(100, 200, 150, 0);
        previous.timestamp = 1000000;
        current.timestamp = 1000001;
        (previous, current)
    }

    fn make_test_process(pid: i32, thread_count: u32, running_threads: u32) -> RawProcessInfo {
        RawProcessInfo {
            pid,
            uid: if pid == 1 { 0 } else { 501 },
            user: if pid == 1 { "root".to_string() } else { "user".to_string() },
            priority: pid * 10,
            nice: 0,
            virtual_size: (pid as u64) * 1024 * 1024,
            resident_size: (pid as u64) * 512 * 1024,
            state: if running_threads > 0 { ProcessState::Running } else { ProcessState::Sleeping },
            cpu_time_ns: (pid as u64) * 1_000_000,
            thread_count,
            running_threads,
            command: format!("/bin/proc{}", pid),
        }
    }

    fn assert_float_eq(a: f64, b: f64, epsilon: f64) {
        let diff = (a - b).abs();
        assert!(
            diff < epsilon,
            "Expected {} to be approximately equal to {} (diff: {})",
            a,
            b,
            diff
        );
    }

    fn assert_percentages_valid(user: f64, system: f64, idle: f64, nice: f64) {
        let sum = user + system + idle + nice;
        assert_float_eq(sum, 100.0, 0.01);
    }

    #[test]
    fn test_snapshot_compute_basic_usage() {
        let previous = make_single_cpu_sample(1000, 500, 8500, 0);
        let current = make_single_cpu_sample(1100, 600, 8700, 0);

        let snapshot = Snapshot::compute(&current, &previous);
        let usage = &snapshot.cpu_usage[0];

        assert_float_eq(usage.user_percent, 25.0, 0.01);
        assert_float_eq(usage.system_percent, 25.0, 0.01);
        assert_float_eq(usage.idle_percent, 50.0, 0.01);
        assert_float_eq(usage.nice_percent, 0.0, 0.01);
        assert_percentages_valid(
            usage.user_percent,
            usage.system_percent,
            usage.idle_percent,
            usage.nice_percent,
        );
    }

    #[test]
    fn test_snapshot_compute_u32_wraparound() {
        let (previous, current) = make_wraparound_sample();

        let snapshot = Snapshot::compute(&current, &previous);
        let usage = &snapshot.cpu_usage[0];

        assert!(usage.user_percent >= 0.0 && usage.user_percent <= 100.0);
        assert!(usage.system_percent >= 0.0 && usage.system_percent <= 100.0);
        assert!(usage.idle_percent >= 0.0 && usage.idle_percent <= 100.0);
        assert!(usage.nice_percent >= 0.0 && usage.nice_percent <= 100.0);
        assert_percentages_valid(
            usage.user_percent,
            usage.system_percent,
            usage.idle_percent,
            usage.nice_percent,
        );
    }

    #[test]
    fn test_snapshot_compute_zero_delta() {
        let sample = make_single_cpu_sample(1000, 500, 8500, 0);

        let snapshot = Snapshot::compute(&sample, &sample);
        let usage = &snapshot.cpu_usage[0];

        assert_float_eq(usage.user_percent, 0.0, 0.001);
        assert_float_eq(usage.system_percent, 0.0, 0.001);
        assert_float_eq(usage.idle_percent, 0.0, 0.001);
        assert_float_eq(usage.nice_percent, 0.0, 0.001);
    }

    #[test]
    fn test_snapshot_compute_multiple_cpus() {
        let previous = make_sample(
            4,
            vec![
                [1000, 500, 8500, 0],
                [2000, 1000, 7000, 0],
                [1500, 750, 7750, 0],
                [3000, 1500, 5500, 0],
            ],
        );
        let current = make_sample(
            4,
            vec![
                [1100, 600, 8800, 0],
                [2200, 1200, 7600, 0],
                [1600, 850, 8050, 0],
                [3300, 1800, 6400, 0],
            ],
        );

        let snapshot = Snapshot::compute(&current, &previous);

        assert_eq!(snapshot.cpu_usage.len(), 4);
        for usage in &snapshot.cpu_usage {
            assert_percentages_valid(
                usage.user_percent,
                usage.system_percent,
                usage.idle_percent,
                usage.nice_percent,
            );
        }
    }

    #[test]
    #[should_panic(expected = "CPU count mismatch")]
    fn test_snapshot_compute_cpu_count_mismatch_panics() {
        let previous = make_sample(2, vec![[1000, 500, 8500, 0], [1000, 500, 8500, 0]]);
        let current = make_sample(
            4,
            vec![
                [1100, 600, 8300, 0],
                [1100, 600, 8300, 0],
                [1100, 600, 8300, 0],
                [1100, 600, 8300, 0],
            ],
        );

        Snapshot::compute(&current, &previous);
    }

    #[test]
    fn test_snapshot_compute_single_active_counter() {
        let previous = make_single_cpu_sample(1000, 500, 8500, 0);
        let current = make_single_cpu_sample(2000, 500, 8500, 0);

        let snapshot = Snapshot::compute(&current, &previous);
        let usage = &snapshot.cpu_usage[0];

        assert_float_eq(usage.user_percent, 100.0, 0.01);
        assert_float_eq(usage.system_percent, 0.0, 0.01);
        assert_float_eq(usage.idle_percent, 0.0, 0.01);
        assert_float_eq(usage.nice_percent, 0.0, 0.01);
    }

    #[test]
    fn test_snapshot_compute_multi_cpu_wraparound() {
        let previous = make_sample(
            4,
            vec![
                [1000, 500, 8500, 0],
                [u32::MAX - 100, 200, 100, 0],
                [500, u32::MAX - 50, 8500, 0],
                [1500, 750, u32::MAX - 200, 0],
            ],
        );
        let current = make_sample(
            4,
            vec![
                [1100, 600, 8700, 0],
                [150, 300, 200, 0],
                [600, 100, 8600, 0],
                [1600, 850, 150, 0],
            ],
        );

        let snapshot = Snapshot::compute(&current, &previous);

        assert_eq!(snapshot.cpu_usage.len(), 4);

        for (i, usage) in snapshot.cpu_usage.iter().enumerate() {
            assert!(
                usage.user_percent >= 0.0 && usage.user_percent <= 100.0,
                "CPU {} user_percent out of range: {}",
                i,
                usage.user_percent
            );
            assert!(
                usage.system_percent >= 0.0 && usage.system_percent <= 100.0,
                "CPU {} system_percent out of range: {}",
                i,
                usage.system_percent
            );
            assert!(
                usage.idle_percent >= 0.0 && usage.idle_percent <= 100.0,
                "CPU {} idle_percent out of range: {}",
                i,
                usage.idle_percent
            );
            assert!(
                usage.nice_percent >= 0.0 && usage.nice_percent <= 100.0,
                "CPU {} nice_percent out of range: {}",
                i,
                usage.nice_percent
            );

            assert_percentages_valid(
                usage.user_percent,
                usage.system_percent,
                usage.idle_percent,
                usage.nice_percent,
            );
        }
    }

    proptest! {
        #[test]
        fn prop_percentages_sum_to_100_or_0(
            prev_user in 0u32..u32::MAX/2,
            prev_system in 0u32..u32::MAX/2,
            prev_idle in 0u32..u32::MAX/2,
            prev_nice in 0u32..u32::MAX/2,
            delta_user in 0u32..1_000_000u32,
            delta_system in 0u32..1_000_000u32,
            delta_idle in 0u32..1_000_000u32,
            delta_nice in 0u32..1_000_000u32,
        ) {
            let previous = make_single_cpu_sample(prev_user, prev_system, prev_idle, prev_nice);
            let current = make_single_cpu_sample(
                prev_user.wrapping_add(delta_user),
                prev_system.wrapping_add(delta_system),
                prev_idle.wrapping_add(delta_idle),
                prev_nice.wrapping_add(delta_nice),
            );

            let snapshot = Snapshot::compute(&current, &previous);
            let usage = &snapshot.cpu_usage[0];

            let sum = usage.user_percent + usage.system_percent + usage.idle_percent + usage.nice_percent;
            prop_assert!(
                (sum - 100.0).abs() < 0.01 || (sum - 0.0).abs() < 0.01,
                "Percentages must sum to 100% or 0%, got {}%", sum
            );
        }

        #[test]
        fn prop_all_percentages_in_valid_range(
            prev_user in 0u32..u32::MAX/2,
            prev_system in 0u32..u32::MAX/2,
            prev_idle in 0u32..u32::MAX/2,
            prev_nice in 0u32..u32::MAX/2,
            delta_user in 0u32..1_000_000u32,
            delta_system in 0u32..1_000_000u32,
            delta_idle in 0u32..1_000_000u32,
            delta_nice in 0u32..1_000_000u32,
        ) {
            let previous = make_single_cpu_sample(prev_user, prev_system, prev_idle, prev_nice);
            let current = make_single_cpu_sample(
                prev_user.wrapping_add(delta_user),
                prev_system.wrapping_add(delta_system),
                prev_idle.wrapping_add(delta_idle),
                prev_nice.wrapping_add(delta_nice),
            );

            let snapshot = Snapshot::compute(&current, &previous);
            let usage = &snapshot.cpu_usage[0];

            prop_assert!(usage.user_percent >= 0.0 && usage.user_percent <= 100.0);
            prop_assert!(usage.system_percent >= 0.0 && usage.system_percent <= 100.0);
            prop_assert!(usage.idle_percent >= 0.0 && usage.idle_percent <= 100.0);
            prop_assert!(usage.nice_percent >= 0.0 && usage.nice_percent <= 100.0);
        }

        #[test]
        fn prop_multi_cpu_percentages_valid(
            cpu_count in 1usize..=16,
            prev_ticks in prop::collection::vec(
                (0u32..u32::MAX/2, 0u32..u32::MAX/2, 0u32..u32::MAX/2, 0u32..u32::MAX/2),
                1..=16
            ),
            deltas in prop::collection::vec(
                (0u32..1_000_000u32, 0u32..1_000_000u32, 0u32..1_000_000u32, 0u32..1_000_000u32),
                1..=16
            )
        ) {
            let len = cpu_count.min(prev_ticks.len()).min(deltas.len());
            let prev_arr: Vec<[u32; 4]> = prev_ticks[..len].iter()
                .map(|(u, s, i, n)| [*u, *s, *i, *n])
                .collect();
            let curr_arr: Vec<[u32; 4]> = prev_ticks[..len].iter()
                .zip(deltas[..len].iter())
                .map(|((u, s, i, n), (du, ds, di, dn))| [
                    u.wrapping_add(*du),
                    s.wrapping_add(*ds),
                    i.wrapping_add(*di),
                    n.wrapping_add(*dn),
                ])
                .collect();

            let previous = make_sample(len, prev_arr);
            let current = make_sample(len, curr_arr);

            let snapshot = Snapshot::compute(&current, &previous);

            prop_assert_eq!(snapshot.cpu_usage.len(), len);

            for usage in &snapshot.cpu_usage {
                let sum = usage.user_percent + usage.system_percent + usage.idle_percent + usage.nice_percent;
                prop_assert!(
                    (sum - 100.0).abs() < 0.01 || (sum - 0.0).abs() < 0.01,
                    "Percentages must sum to 100% or 0%, got {}%", sum
                );

                prop_assert!(usage.user_percent >= 0.0 && usage.user_percent <= 100.0);
                prop_assert!(usage.system_percent >= 0.0 && usage.system_percent <= 100.0);
                prop_assert!(usage.idle_percent >= 0.0 && usage.idle_percent <= 100.0);
                prop_assert!(usage.nice_percent >= 0.0 && usage.nice_percent <= 100.0);
            }
        }
    }

    #[test]
    fn test_task_stats_default() {
        let task_stats = TaskStats::default();
        assert_eq!(task_stats.total_tasks, 0);
        assert_eq!(task_stats.total_threads, 0);
        assert_eq!(task_stats.running_threads, 0);
    }

    #[test]
    fn test_snapshot_derives_task_stats_from_processes() {
        let previous = make_single_cpu_sample(1000, 500, 8500, 0);
        let mut current = make_single_cpu_sample(1100, 600, 8700, 0);

        current.processes = vec![
            make_test_process(1, 10, 1),
            make_test_process(2, 20, 2),
            make_test_process(3, 30, 3),
        ];

        let snapshot = Snapshot::compute(&current, &previous);

        assert_eq!(snapshot.task_stats.total_tasks, 3);
        assert_eq!(snapshot.task_stats.total_threads, 60);
        assert_eq!(snapshot.task_stats.running_threads, 6);
    }

    proptest! {
        #[test]
        fn prop_task_stats_threads_ge_tasks(
            total_tasks in 0u32..10_000u32,
            extra_threads in 0u32..100_000u32,
            running in 0u32..100_000u32,
        ) {
            let total_threads = total_tasks + extra_threads;
            let running_threads = running.min(total_threads);

            let task_stats = TaskStats {
                total_tasks,
                total_threads,
                running_threads,
            };

            prop_assert!(task_stats.total_threads >= task_stats.total_tasks,
                "total_threads ({}) must be >= total_tasks ({})",
                task_stats.total_threads, task_stats.total_tasks);

            prop_assert!(task_stats.running_threads <= task_stats.total_threads,
                "running_threads ({}) must be <= total_threads ({})",
                task_stats.running_threads, task_stats.total_threads);
        }
    }
}
