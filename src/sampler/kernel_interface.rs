use crate::model::RawSample;

pub trait KernelInterface {
    fn get_processor_info(&self) -> Result<RawSample, i32>;
}
