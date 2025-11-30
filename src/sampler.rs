pub mod kernel_interface;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxSampler as PlatformSampler;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacOsSampler as PlatformSampler;

use crate::model::RawSample;

pub trait Sampler {
    fn sample(&mut self) -> Result<RawSample, SampleError>;
}

#[derive(Debug, thiserror::Error)]
pub enum SampleError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("System error code: {0}")]
    System(i32),
}
