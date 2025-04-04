//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, get_current_syscall_cnt, suspend_current_and_run_next},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    match trace_request {
        0 => {
            // 如果 trace_request 为 0
            // 则 id 应被视作 *const u8
            // 表示读取当前任务 id 地址处一个字节的无符号整数值
            // 此时应忽略 data 参数。返回值为 id 地址处的值
            unsafe { (id as *const u8).read_volatile() as isize }
        }
        1 => {
            // 如果 trace_request 为 1
            // 则 id 应被视作 *const u8
            // 表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0
            unsafe { *(id as *mut u8) = data as u8 }
            0
        }
        2 => {
            // 如果 trace_request 为 2
            // 表示查询 *当前任务* 调用编号为 id 的系统调用的次数
            // 返回值为这个调用次数。*本次调用也计入统计*
            get_current_syscall_cnt(id) as isize
        }
        _ => -1,
    }
}
