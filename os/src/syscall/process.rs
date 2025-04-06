//! Process management syscalls
use core::mem::size_of;

use crate::{
    mm::{translated_byte_buffer, MapPermission, PageTable, VirtAddr},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_current_syscall_cnt,
        mmap_current, munmap_current, suspend_current_and_run_next,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");

    let us = get_time_us();
    let timeval = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    let mut timeval = &timeval as *const TimeVal as *const u8;

    let vecs = translated_byte_buffer(current_user_token(), ts as *const u8, size_of::<TimeVal>());
    for vec in vecs {
        let s = vec.len();
        unsafe {
            timeval.copy_to(vec.as_mut_ptr(), s);
            timeval = timeval.offset(s as isize)
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    if trace_request == 0 || trace_request == 1 {
        let va = VirtAddr::from(id);
        let page_table = PageTable::from_token(current_user_token());
        if let Some(pte) = page_table.translate(va.floor()) {
            if !pte.is_valid() || !pte.is_user() {
                return -1;
            }
            if trace_request == 0 {
                // read
                if pte.readable() {
                    return pte.ppn().get_bytes_array()[va.page_offset()] as isize;
                } else {
                    return -1;
                }
            } else {
                // write
                if pte.writable() {
                    pte.ppn().get_bytes_array()[va.page_offset()] = data as u8;
                    return 0;
                } else {
                    return -1;
                }
            }
        } else {
            return -1;
        }
    } else if trace_request == 2 {
        return get_current_syscall_cnt(id) as isize;
    } else {
        return -1;
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start_va = VirtAddr::from(start);
    if !start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr::from(start + len);
    if prot == 0 || (prot & !0x7 != 0) {
        return -1;
    }
    // when wirte bit is set but read bit is not set
    if prot & 0x1 << 1 == 1 && prot & 0x1 == 0 {
        return -1;
    }

    let mut permission = MapPermission::U;
    if (prot & 0x1) != 0 {
        permission |= MapPermission::R;
    }
    if (prot & 0x1 << 1) != 0 {
        permission |= MapPermission::W;
    }
    if (prot & 0x1 << 2) != 0 {
        permission |= MapPermission::X;
    }

    if mmap_current(start_va, end_va, permission) {
        return 0;
    } else {
        return -1;
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    let start_va = VirtAddr::from(start);
    if !start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr::from(start + len);
    if munmap_current(start_va, end_va) {
        0
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
