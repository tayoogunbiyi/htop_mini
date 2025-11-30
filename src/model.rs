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