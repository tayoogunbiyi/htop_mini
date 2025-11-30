use htop_mini::sampler::kernel_interface::KernelInterface;
use htop_mini::model::RawSample;
use std::collections::VecDeque;

pub struct MockKernel {
    responses: VecDeque<Result<RawSample, i32>>,
}

impl MockKernel {
    pub fn new() -> Self {
        MockKernel {
            responses: VecDeque::new(),
        }
    }

    pub fn push_success(&mut self, sample: RawSample) {
        self.responses.push_back(Ok(sample));
    }

    pub fn push_error(&mut self, error_code: i32) {
        self.responses.push_back(Err(error_code));
    }
}

impl KernelInterface for MockKernel {
    fn get_processor_info(&self) -> Result<RawSample, i32> {
        // TODO: This uses .front() (peek) instead of .pop_front() (consume)
        // because the trait requires &self (immutable). This means repeated
        // calls return the same value. If tests need to consume the queue,
        // switch to RefCell<VecDeque<...>> for interior mutability.
        self.responses.front()
            .cloned()
            .expect("MockKernel: No more responses configured")
    }
}
