#![allow(non_camel_case_types)]
use super::kernel_interface::KernelInterface;
use super::{SampleError, Sampler};
use crate::model::{BootInfo, CpuTicks, LoadAverage, MemoryStats, RawSample};

use std::mem;
use std::ptr;

use mach::kern_return::kern_return_t;
use mach::message::mach_msg_type_number_t;
use mach::port::mach_port_t;
use mach::traps::mach_task_self;
use mach::vm::mach_vm_deallocate;
use mach::vm_types::{mach_vm_address_t, mach_vm_size_t, natural_t};

use libc::{CTL_HW, CTL_KERN, CTL_VM, KERN_BOOTTIME, c_void, getloadavg, sysctl, timeval};

type host_t = mach_port_t;
type host_flavor_t = i32;
type processor_info_array_t = *mut i32;

const PROCESSOR_CPU_LOAD_INFO: host_flavor_t = 2;
const CPU_STATE_MAX: usize = 4;
const CPU_STATE_USER: usize = 0;
const CPU_STATE_SYSTEM: usize = 1;
const CPU_STATE_IDLE: usize = 2;
const CPU_STATE_NICE: usize = 3;

const VM_SWAPUSAGE: i32 = 5;
const HW_MEMSIZE: i32 = 24;
const HOST_VM_INFO64: i32 = 4;

// swap storage
#[repr(C)]
struct xsw_usage {
    xsu_total: u64,
    xsu_avail: u64,
    xsu_used: u64,
    xsu_pagesize: u32,
    xsu_encrypted: bool,
}

#[repr(C, align(8))]
struct vm_statistics64 {
    free_count: natural_t,
    active_count: natural_t,
    inactive_count: natural_t,
    wire_count: natural_t,
    zero_fill_count: u64,
    reactivations: u64,
    pageins: u64,
    pageouts: u64,
    faults: u64,
    cow_faults: u64,
    lookups: u64,
    hits: u64,
    purges: u64,
    purgeable_count: natural_t,
    speculative_count: natural_t,
    decompressions: u64,
    compressions: u64,
    swapins: u64,
    swapouts: u64,
    compressor_page_count: natural_t,
    throttled_count: natural_t,
    external_page_count: natural_t,
    internal_page_count: natural_t,
    total_uncompressed_pages_in_compressor: u64,
}

type vm_size_t = usize;

unsafe extern "C" {
    fn mach_host_self() -> host_t;

    fn host_processor_info(
        host: host_t,
        flavor: host_flavor_t,
        out_processor_count: *mut natural_t,
        out_processor_info: *mut processor_info_array_t,
        out_processor_info_count: *mut mach_msg_type_number_t,
    ) -> kern_return_t;

    fn host_page_size(host: host_t, page_size: *mut vm_size_t) -> kern_return_t;

    fn host_statistics64(
        host: host_t,
        flavor: host_flavor_t,
        host_info: *mut vm_statistics64,
        count: *mut mach_msg_type_number_t,
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

            let boot_info = self.get_boot_info()?;
            let load_average = self.get_load_average()?;
            let memory_stats = self.get_memory_stats()?;

            Ok(RawSample {
                cpu_count: processor_count as usize,
                cpu_ticks,
                boot_info,
                load_average,
                memory_stats,
            })
        }
    }

    fn get_boot_info(&self) -> Result<BootInfo, i32> {
        unsafe {
            let mut boottime: timeval = mem::zeroed();
            let mut len = mem::size_of::<timeval>();
            let mut mib = [CTL_KERN, KERN_BOOTTIME];

            let result = sysctl(
                mib.as_mut_ptr(),
                2,
                &mut boottime as *mut _ as *mut c_void,
                &mut len,
                ptr::null_mut(),
                0,
            );

            if result != 0 {
                return Err(result);
            }

            Ok(BootInfo {
                boot_time_secs: boottime.tv_sec as u64,
            })
        }
    }

    fn get_load_average(&self) -> Result<LoadAverage, i32> {
        unsafe {
            let mut loadavg = [0.0f64; 3];
            let result = getloadavg(loadavg.as_mut_ptr(), 3);

            if result != 3 {
                return Err(-1);
            }

            Ok(LoadAverage {
                one_min: loadavg[0],
                five_min: loadavg[1],
                fifteen_min: loadavg[2],
            })
        }
    }

    fn get_memory_stats(&self) -> Result<MemoryStats, i32> {
        unsafe {
            let host = mach_host_self();

            let mut page_size: vm_size_t = 0;
            let result = host_page_size(host, &mut page_size);
            if result != 0 {
                return Err(result);
            }

            let mut vm_stats: vm_statistics64 = mem::zeroed();
            let mut count = (mem::size_of::<vm_statistics64>() / mem::size_of::<i32>()) as u32;

            let result =
                host_statistics64(host, HOST_VM_INFO64, &mut vm_stats as *mut _, &mut count);
            if result != 0 {
                return Err(result);
            }

            let mut total_mem: u64 = 0;
            let mut len = mem::size_of::<u64>();
            let mut mib = [CTL_HW, HW_MEMSIZE];

            let result = sysctl(
                mib.as_mut_ptr(),
                2,
                &mut total_mem as *mut _ as *mut c_void,
                &mut len,
                ptr::null_mut(),
                0,
            );
            if result != 0 {
                return Err(result);
            }

            let mut swap_info: xsw_usage = mem::zeroed();
            let mut len = mem::size_of::<xsw_usage>();
            let mut mib = [CTL_VM, VM_SWAPUSAGE];

            let result = sysctl(
                mib.as_mut_ptr(),
                2,
                &mut swap_info as *mut _ as *mut c_void,
                &mut len,
                ptr::null_mut(),
                0,
            );
            if result != 0 {
                return Err(result);
            }

            Ok(MemoryStats {
                total_memory_bytes: total_mem,
                active_bytes: vm_stats.active_count as u64 * page_size as u64,
                inactive_bytes: vm_stats.inactive_count as u64 * page_size as u64,
                wired_bytes: vm_stats.wire_count as u64 * page_size as u64,
                compressed_bytes: vm_stats.total_uncompressed_pages_in_compressor * page_size as u64,
                free_bytes: vm_stats.free_count as u64 * page_size as u64,
                purgeable_bytes: vm_stats.purgeable_count as u64 * page_size as u64,
                page_size: page_size as u64,
                swap_total_bytes: swap_info.xsu_total,
                swap_used_bytes: swap_info.xsu_used,
            })
        }
    }
}
