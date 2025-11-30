#![allow(non_camel_case_types)]

use super::{Sampler, SampleError};
use crate::model::{RawSample, CpuTicks};

use std::mem;
use std::ptr;


type kern_return_t = i32;
type host_t = u32;
type host_flavor_t = i32;
type natural_t = u32;
type processor_info_array_t = *mut i32;
type mach_msg_type_number_t = u32;

const PROCESSOR_CPU_LOAD_INFO: host_flavor_t = 2;
const CPU_STATE_MAX: usize = 4;
const CPU_STATE_USER: usize = 0;
const CPU_STATE_SYSTEM: usize = 1;
const CPU_STATE_IDLE: usize = 2;
const CPU_STATE_NICE: usize = 3;

#[link(name = "System", kind = "framework")]
unsafe extern "C" {
    fn mach_host_self() -> host_t;
    
    fn host_processor_info(
        host: host_t,
        flavor: host_flavor_t,
        out_processor_count: *mut natural_t,
        out_processor_info: *mut processor_info_array_t,
        out_processor_info_count: *mut mach_msg_type_number_t,
    ) -> kern_return_t;
    
    fn vm_deallocate(
        target_task: u32,
        address: u64,
        size: u64,
    ) -> kern_return_t;
    
    fn mach_task_self() -> u32;
}


pub struct MacOsSampler {
}

impl MacOsSampler {
    pub fn new() -> Self {
        MacOsSampler {}
    }

    fn collect_cpu_info(&mut self) -> Result<RawSample, SampleError> {
        unsafe {
            let host = mach_host_self();
            let mut processor_count: natural_t = 0;
            let mut processor_info: processor_info_array_t = ptr::null_mut();
            let mut processor_info_count: mach_msg_type_number_t = 0;

            let result = host_processor_info(
                host,
                PROCESSOR_CPU_LOAD_INFO,
                &mut processor_count,
                &mut processor_info,
                &mut processor_info_count,
            );

            if result != 0 {
                return Err(SampleError::System(result))
            }

            let cpu_info = std::slice::from_raw_parts(
                processor_info,
                processor_info_count as usize,
            );

            let mut cpu_ticks = Vec::with_capacity(processor_count as usize);

            for i in 0..processor_count as usize {
                let offset = i * CPU_STATE_MAX;
                let user = cpu_info[offset + CPU_STATE_USER] as u32;
                let system = cpu_info[offset + CPU_STATE_SYSTEM] as u32;
                let idle = cpu_info[offset + CPU_STATE_IDLE] as u32;
                let nice = cpu_info[offset + CPU_STATE_NICE] as u32;

                cpu_ticks.push(CpuTicks {
                    user,
                    system,
                    idle,
                    nice,
                });

            }

            vm_deallocate(
                mach_task_self(),
                processor_info as u64,
                (processor_info_count * mem::size_of::<i32>() as u32) as u64,
            );

            Ok(RawSample {
                cpu_count: processor_count as usize,
                cpu_ticks,
            })
        }
    }
}

impl Sampler for MacOsSampler {
    fn sample(&mut self) -> Result<RawSample, SampleError> {
        self.collect_cpu_info()
    }
}