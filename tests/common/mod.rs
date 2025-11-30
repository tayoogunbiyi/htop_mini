
pub mod mock_kernel;

use htop_mini::model::{RawSample, CpuTicks};

pub fn make_single_cpu_sample(user: u32, system: u32, idle: u32, nice: u32) -> RawSample {
    make_sample(1, vec![[user, system, idle, nice]])
}

pub fn make_sample(cpu_count: usize, ticks: Vec<[u32; 4]>) -> RawSample {
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

pub fn make_wraparound_sample() -> (RawSample, RawSample) {
    let previous = make_single_cpu_sample(u32::MAX - 50, 100, 100, 0);
    let current = make_single_cpu_sample(100, 200, 150, 0);
    (previous, current)
}

#[allow(dead_code)]
pub fn assert_float_eq(a: f64, b: f64, epsilon: f64) {
    let diff = (a - b).abs();
    assert!(diff <= epsilon, "Expected {} to be approximately equal to {} (diff: {})", a, b, diff);
}

#[allow(dead_code)]
pub fn assert_percentages_valid(user: f64, system: f64, idle: f64, nice: f64) {
    let sum = user + system + idle + nice;
    assert_float_eq(sum, 100.0, 0.01);
}
