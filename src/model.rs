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

                let total_delta = delta_user
                    .wrapping_add(delta_system)
                    .wrapping_add(delta_idle)
                    .wrapping_add(delta_nice);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_single_cpu_sample(user: u32, system: u32, idle: u32, nice: u32) -> RawSample {
        RawSample {
            cpu_count: 1,
            cpu_ticks: vec![CpuTicks { user, system, idle, nice }],
        }
    }

    fn make_sample(cpu_count: usize, ticks: Vec<[u32; 4]>) -> RawSample {
        RawSample {
            cpu_count,
            cpu_ticks: ticks.iter().map(|[u, s, i, n]| CpuTicks {
                user: *u,
                system: *s,
                idle: *i,
                nice: *n,
            }).collect(),
        }
    }

    fn make_wraparound_sample() -> (RawSample, RawSample) {
        let previous = make_single_cpu_sample(u32::MAX - 50, 100, 100, 0);
        let current = make_single_cpu_sample(100, 200, 150, 0);
        (previous, current)
    }

    fn assert_float_eq(a: f64, b: f64, epsilon: f64) {
        let diff = (a - b).abs();
        assert!(diff < epsilon, "Expected {} to be approximately equal to {} (diff: {})", a, b, diff);
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
            usage.nice_percent
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
            usage.nice_percent
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
        let previous = make_sample(4, vec![
            [1000, 500, 8500, 0],
            [2000, 1000, 7000, 0],
            [1500, 750, 7750, 0],
            [3000, 1500, 5500, 0],
        ]);
        let current = make_sample(4, vec![
            [1100, 600, 8800, 0],
            [2200, 1200, 7600, 0],
            [1600, 850, 8050, 0],
            [3300, 1800, 6400, 0],
        ]);

        let snapshot = Snapshot::compute(&current, &previous);

        assert_eq!(snapshot.cpu_usage.len(), 4);
        for usage in &snapshot.cpu_usage {
            assert_percentages_valid(
                usage.user_percent,
                usage.system_percent,
                usage.idle_percent,
                usage.nice_percent
            );
        }
    }

    #[test]
    #[should_panic(expected = "CPU count mismatch")]
    fn test_snapshot_compute_cpu_count_mismatch_panics() {
        let previous = make_sample(2, vec![[1000, 500, 8500, 0], [1000, 500, 8500, 0]]);
        let current = make_sample(4, vec![
            [1100, 600, 8300, 0],
            [1100, 600, 8300, 0],
            [1100, 600, 8300, 0],
            [1100, 600, 8300, 0],
        ]);

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
        let previous = make_sample(4, vec![
            [1000, 500, 8500, 0],
            [u32::MAX - 100, 200, 100, 0],
            [500, u32::MAX - 50, 8500, 0],
            [1500, 750, u32::MAX - 200, 0],
        ]);
        let current = make_sample(4, vec![
            [1100, 600, 8700, 0],
            [150, 300, 200, 0],
            [600, 100, 8600, 0],
            [1600, 850, 150, 0],
        ]);

        let snapshot = Snapshot::compute(&current, &previous);

        assert_eq!(snapshot.cpu_usage.len(), 4);

        for (i, usage) in snapshot.cpu_usage.iter().enumerate() {
            assert!(usage.user_percent >= 0.0 && usage.user_percent <= 100.0,
                "CPU {} user_percent out of range: {}", i, usage.user_percent);
            assert!(usage.system_percent >= 0.0 && usage.system_percent <= 100.0,
                "CPU {} system_percent out of range: {}", i, usage.system_percent);
            assert!(usage.idle_percent >= 0.0 && usage.idle_percent <= 100.0,
                "CPU {} idle_percent out of range: {}", i, usage.idle_percent);
            assert!(usage.nice_percent >= 0.0 && usage.nice_percent <= 100.0,
                "CPU {} nice_percent out of range: {}", i, usage.nice_percent);

            assert_percentages_valid(
                usage.user_percent,
                usage.system_percent,
                usage.idle_percent,
                usage.nice_percent
            );
        }
    }
}