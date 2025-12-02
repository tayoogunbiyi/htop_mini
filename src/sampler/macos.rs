#![allow(non_camel_case_types)]
use super::kernel_interface::KernelInterface;
use super::{SampleError, Sampler};
use crate::model::{CpuTicks, RawSample};

use std::mem;
use std::ptr;

use mach::kern_return::kern_return_t;
use mach::message::mach_msg_type_number_t;
use mach::port::mach_port_t;
use mach::traps::mach_task_self;
use mach::vm::mach_vm_deallocate;
use mach::vm_types::{mach_vm_address_t, mach_vm_size_t, natural_t};

type host_t = mach_port_t;
type host_flavor_t = i32;
type processor_info_array_t = *mut i32;

const PROCESSOR_CPU_LOAD_INFO: host_flavor_t = 2;
const CPU_STATE_MAX: usize = 4;
const CPU_STATE_USER: usize = 0;
const CPU_STATE_SYSTEM: usize = 1;
const CPU_STATE_IDLE: usize = 2;
const CPU_STATE_NICE: usize = 3;

unsafe extern "C" {
    fn mach_host_self() -> host_t;

    fn host_processor_info(
        host: host_t,
        flavor: host_flavor_t,
        out_processor_count: *mut natural_t,
        out_processor_info: *mut processor_info_array_t,
        out_processor_info_count: *mut mach_msg_type_number_t,
    ) -> kern_return_t;
}

pub struct MacOsSampler {
    kernel: Box<dyn KernelInterface>,
}

impl MacOsSampler {
    pub fn new() -> Self {
        MacOsSampler {
            kernel: Box::new(MachKernel),
        }
    }
}

impl Sampler for MacOsSampler {
    fn sample(&mut self) -> Result<RawSample, SampleError> {
        self.kernel
            .get_processor_info()
            .map_err(SampleError::System)
    }
}

pub struct MachKernel;

impl KernelInterface for MachKernel {
    fn get_processor_info(&self) -> Result<RawSample, i32> {
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
                return Err(result);
            }

            let cpu_info =
                std::slice::from_raw_parts(processor_info, processor_info_count as usize);

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

            // CRITICAL: Must call mach_vm_deallocate to prevent memory leaks
            // The Mach kernel API allocates memory that must be manually freed
            // Verify no leaks with: cargo instruments --template Leaks
            mach_vm_deallocate(
                mach_task_self(),
                processor_info as mach_vm_address_t,
                (processor_info_count * mem::size_of::<i32>() as u32) as mach_vm_size_t,
            );

            Ok(RawSample {
                cpu_count: processor_count as usize,
                cpu_ticks,
            })
        }
    }
}
