use crate::model::{BootInfo, LoadAverage, MemoryStats, RawSample};

pub trait KernelInterface {
    fn get_processor_info(&self) -> Result<RawSample, i32>;
    fn get_boot_info(&self) -> Result<BootInfo, i32>;
    fn get_load_average(&self) -> Result<LoadAverage, i32>;
    fn get_memory_stats(&self) -> Result<MemoryStats, i32>;
}
