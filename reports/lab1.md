

## 如何追踪各个用户进程的 syscall 调用数量  

用户程序调用 ecall 流程

- U 态
- 调用 `ecall`
- S 态
- 陷入 `__alltraps`
    - 保存用户程序数据
- `__alltraps` 调用 `trap_handler` 
- `trap_handler` 调用 `Rust` 编写的 `syscall` 函数

## 我需要做什么？

- 维护每个用户进程的 `syscall` 调用数
    - 可以在 `TaskControlBlock` 中维护
- 在 `trap_handler` 对用户进程系的系统调用数量进行统计