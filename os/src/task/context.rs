//! Implementation of [`TaskContext`]

#[derive(Copy, Clone)]
#[repr(C)]
/// task context structure containing some registers
pub struct TaskContext {
    /// Ret position after task switching
    ra: usize,
    /// Stack pointer
    sp: usize,
    /// s0-11 register, callee saved
    s: [usize; 12],
}

impl TaskContext {
    /// Create a new empty task context
    pub fn zero_init() -> Self {
        Self {
            // 什么是 ra 寄存器？
            // 在 risc-v 中调用 ret 就会跳转到 ra 寄存器中保存的指令的地址处开始执行
            ra: 0,
            sp: 0,
            // “保存寄存器（save register）” 的状态 s0-s11
            s: [0; 12],
        }
    }
    /// Create a new task context with a trap return addr and a kernel stack pointer
    pub fn goto_restore(kstack_ptr: usize) -> Self {
        extern "C" {
            fn __restore();
        }
        Self {
            ra: __restore as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
