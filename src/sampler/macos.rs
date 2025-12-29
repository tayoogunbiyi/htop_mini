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

use libc::{CTL_HW, CTL_KERN, CTL_VM, KERN_BOOTTIME};
use libc::{c_char, c_void, getloadavg, getpwuid, sysctl, timeval, uid_t};

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

const PROC_PIDTASKINFO: i32 = 4;
const PROC_PIDPATHINFO_MAXSIZE: usize = 4096;

const KERN_PROC: i32 = 14;
const KERN_PROC_ALL: i32 = 0;

const SZOMB: i8 = 5;
const SSTOP: i8 = 4;

#[repr(C)]
struct extern_proc {
    p_un: [u8; 16],
    p_vmspace: *mut c_void,
    p_sigacts: *mut c_void,
    p_flag: i32,
    p_stat: i8,
    p_pid: i32,
    p_oppid: i32,
    p_dupfd: i32,
    user_stack: *mut c_void,
    exit_thread: *mut c_void,
    p_debugger: i32,
    sigwait: i32,
    p_estcpu: u32,
    p_cpticks: i32,
    p_pctcpu: u32,
    p_wchan: *mut c_void,
    p_wmesg: *mut c_char,
    p_swtime: u32,
    p_slptime: u32,
    p_realtimer: [u8; 32],
    p_rtime: timeval,
    p_uticks: u64,
    p_sticks: u64,
    p_iticks: u64,
    p_traceflag: i32,
    p_tracep: *mut c_void,
    p_siglist: i32,
    p_textvp: *mut c_void,
    p_holdcnt: i32,
    p_sigmask: u32,
    p_sigignore: u32,
    p_sigcatch: u32,
    p_priority: u8,
    p_usrpri: u8,
    p_nice: i8,
    p_comm: [u8; 17],
    p_pgrp: *mut c_void,
    p_addr: *mut c_void,
    p_xstat: u16,
    p_acflag: u16,
    p_ru: *mut c_void,
}

#[repr(C)]
struct eproc {
    e_paddr: *mut c_void,
    e_sess: *mut c_void,
    e_pcred: [u8; 104],
    e_ucred: eproc_ucred,
    _pad1: [u8; 4],
    e_vm: [u8; 64],
    e_ppid: i32,
    e_pgid: i32,
    e_jobc: i16,
    _pad2: i16,
    e_tdev: i32,
    e_tpgid: i32,
    _pad3: i32,
    e_tsess: *mut c_void,
    e_wmesg: [u8; 8],
    e_xsize: i32,
    e_xrssize: i16,
    e_xccount: i16,
    e_xswrss: i16,
    _pad4: i16,
    e_flag: i32,
    e_login: [u8; 12],
    e_spare: [i32; 4],
    _pad5: i32,
}

#[repr(C)]
struct eproc_ucred {
    cr_ref: i32,
    cr_uid: uid_t,
    cr_ngroups: i16,
    _pad: i16,
    cr_groups: [u32; 16],
}

#[repr(C)]
struct kinfo_proc {
    kp_proc: extern_proc,
    kp_eproc: eproc,
}

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

#[repr(C)]
struct proc_taskinfo {
    pti_virtual_size: u64,
    pti_resident_size: u64,
    pti_total_user: u64,
    pti_total_system: u64,
    pti_threads_user: u64,
    pti_threads_system: u64,
    pti_policy: i32,
    pti_faults: i32,
    pti_pageins: i32,
    pti_cow_faults: i32,
    pti_messages_sent: i32,
    pti_messages_received: i32,
    pti_syscalls_mach: i32,
    pti_syscalls_unix: i32,
    pti_csw: i32,
    pti_threadnum: i32,
    pti_numrunning: i32,
    pti_priority: i32,
}

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

    fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buffer: *mut c_void, buffersize: i32) -> i32;

    fn proc_pidpath(pid: i32, buffer: *mut c_void, buffersize: u32) -> i32;
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

            mach_vm_deallocate(
                mach_task_self(),
                processor_info as mach_vm_address_t,
                (processor_info_count * mem::size_of::<i32>() as u32) as mach_vm_size_t,
            );

            let boot_info = self.get_boot_info()?;
            let load_average = self.get_load_average()?;
            let memory_stats = self.get_memory_stats()?;
            let processes = self.get_processes().unwrap_or_default();

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            Ok(RawSample {
                timestamp,
                cpu_count: processor_count as usize,
                cpu_ticks,
                boot_info,
                load_average,
                memory_stats,
                processes,
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
                compressed_bytes: vm_stats.compressor_page_count as u64 * page_size as u64,
                free_bytes: vm_stats.free_count as u64 * page_size as u64,
                purgeable_bytes: vm_stats.purgeable_count as u64 * page_size as u64,
                page_size: page_size as u64,
                swap_total_bytes: swap_info.xsu_total,
                swap_used_bytes: swap_info.xsu_used,
            })
        }
    }

    fn get_processes(&self) -> Result<Vec<crate::model::RawProcessInfo>, i32> {
        unsafe {
            let mut mib = [CTL_KERN, KERN_PROC, KERN_PROC_ALL, 0];
            let mut size: usize = 0;

            let result = sysctl(
                mib.as_mut_ptr(),
                3,
                ptr::null_mut(),
                &mut size,
                ptr::null_mut(),
                0,
            );
            if result != 0 {
                return Err(result);
            }

            size += 16 * mem::size_of::<kinfo_proc>();
            let count = size / mem::size_of::<kinfo_proc>();
            let mut kprocs: Vec<kinfo_proc> = Vec::with_capacity(count);
            kprocs.set_len(count);

            let result = sysctl(
                mib.as_mut_ptr(),
                3,
                kprocs.as_mut_ptr() as *mut c_void,
                &mut size,
                ptr::null_mut(),
                0,
            );
            if result != 0 {
                return Err(result);
            }

            let actual_count = size / mem::size_of::<kinfo_proc>();
            kprocs.truncate(actual_count);

            let mut processes = Vec::with_capacity(actual_count);

            for kp in &kprocs {
                let pid = kp.kp_proc.p_pid;
                if pid <= 0 {
                    continue;
                }

                let uid = kp.kp_eproc.e_ucred.cr_uid;
                let user = {
                    let pw = getpwuid(uid as uid_t);
                    if !pw.is_null() && !(*pw).pw_name.is_null() {
                        std::ffi::CStr::from_ptr((*pw).pw_name as *const c_char)
                            .to_string_lossy()
                            .to_string()
                    } else {
                        uid.to_string()
                    }
                };

                let comm_len = kp
                    .kp_proc
                    .p_comm
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(kp.kp_proc.p_comm.len());
                let comm = String::from_utf8_lossy(&kp.kp_proc.p_comm[..comm_len]).to_string();

                let mut taskinfo: proc_taskinfo = mem::zeroed();
                let task_result = proc_pidinfo(
                    pid,
                    PROC_PIDTASKINFO,
                    0,
                    &mut taskinfo as *mut _ as *mut c_void,
                    mem::size_of::<proc_taskinfo>() as i32,
                );
                let has_task_info = task_result > 0;

                let command = if has_task_info {
                    let mut path_buffer = vec![0u8; PROC_PIDPATHINFO_MAXSIZE];
                    let path_result = proc_pidpath(
                        pid,
                        path_buffer.as_mut_ptr() as *mut c_void,
                        PROC_PIDPATHINFO_MAXSIZE as u32,
                    );
                    if path_result > 0 {
                        let path_len = path_buffer
                            .iter()
                            .position(|&b| b == 0)
                            .unwrap_or(path_result as usize);
                        String::from_utf8_lossy(&path_buffer[..path_len]).to_string()
                    } else {
                        comm.clone()
                    }
                } else {
                    comm.clone()
                };

                let state = match kp.kp_proc.p_stat {
                    SZOMB => crate::model::ProcessState::Zombie,
                    SSTOP => crate::model::ProcessState::Stopped,
                    _ => {
                        if has_task_info && taskinfo.pti_numrunning > 0 {
                            crate::model::ProcessState::Running
                        } else {
                            crate::model::ProcessState::Sleeping
                        }
                    }
                };

                let (virtual_size, resident_size, cpu_time_ns, thread_count, running_threads, priority) =
                    if has_task_info {
                        (
                            taskinfo.pti_virtual_size,
                            taskinfo.pti_resident_size,
                            taskinfo.pti_total_user + taskinfo.pti_total_system,
                            taskinfo.pti_threadnum as u32,
                            taskinfo.pti_numrunning as u32,
                            taskinfo.pti_priority,
                        )
                    } else {
                        (0, 0, 0, 1, 0, kp.kp_proc.p_priority as i32)
                    };

                processes.push(crate::model::RawProcessInfo {
                    pid,
                    uid,
                    user,
                    priority,
                    nice: kp.kp_proc.p_nice as i32,
                    virtual_size,
                    resident_size,
                    state,
                    cpu_time_ns,
                    thread_count,
                    running_threads,
                    command,
                });
            }

            Ok(processes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_processes_returns_valid_data() {
        let kernel = MachKernel;
        let result = kernel.get_processes();

        assert!(result.is_ok(), "get_processes should succeed");

        let processes = result.unwrap();

        assert!(!processes.is_empty(), "should have at least some processes");

        let total_threads: u32 = processes.iter().map(|p| p.thread_count).sum();
        assert!(total_threads > 0, "should have at least some threads");
        assert!(
            total_threads >= processes.len() as u32,
            "total_threads ({}) should be >= total_tasks ({})",
            total_threads,
            processes.len()
        );

        let running_threads: u32 = processes.iter().map(|p| p.running_threads).sum();
        assert!(
            running_threads <= total_threads,
            "running_threads ({}) should be <= total_threads ({})",
            running_threads,
            total_threads
        );
    }

    #[test]
    fn test_get_processes_consistent_across_calls() {
        let kernel = MachKernel;

        let result1 = kernel.get_processes();
        let result2 = kernel.get_processes();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let processes1 = result1.unwrap();
        let processes2 = result2.unwrap();

        let diff = (processes1.len() as i32 - processes2.len() as i32).abs();
        assert!(
            diff < 100,
            "process count should be relatively stable between calls (diff: {})",
            diff
        );
    }

    #[test]
    fn test_struct_sizes() {
        assert_eq!(std::mem::size_of::<kinfo_proc>(), 648, "kinfo_proc size mismatch");
        assert_eq!(std::mem::size_of::<extern_proc>(), 296, "extern_proc size mismatch");
        assert_eq!(std::mem::size_of::<eproc>(), 352, "eproc size mismatch");
    }
}
