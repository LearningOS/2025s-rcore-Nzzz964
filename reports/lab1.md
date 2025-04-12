# 实验报告

当 `trace_request` 为 0 和 1 时的功能很简单，这里不赘述。

当 `trace_request` 为 2 时，查询用户程序调用不同的系统调用的个数。这里在 `TaskControlBlock` 维护了 `[usize; 500]` 的静态数组维护不同系统调用的个数。并在 `syscall` 将对应的的调用数 `+1`


# 简答作业

> Q: 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

A: `ch2b_bad_address` 程序会触发 `StoreFault`，同时 `stval` 保存用户程序试图访问的地址，`sepc` 保存了触发 `StoreFault` 的指令的地址  

`ch2b_bad_register` 和 `ch2b_bad_instructions` 都是试图在 U 态访问 S 态的寄存器或者只有在 S 态才能使用的指令，这会触发 `IllegalInstruction`。  

这些错误都会使 CPU 陷入 Trap  

---

深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:

> Q1: L40：刚进入 __restore 时，sp 代表了什么值。请指出 __restore 的两种使用情景。

A: 刚进入内核时 `sp` 代表了 `Kernel Stack` 的栈顶  

`__restore` 是内核从 S 态返回 U 态的统一的入口。这包含两种情况：

1. 初始化用户程序时（初次运行用户程序时）
2. 用户程序访问系统调用时返回时

> Q2: L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

A: 处理了 `sstatus`、`sepc`、`sscratch`，其中:

- `sepc` 保存了执行 `sret` 指令后 CPU 下一条需要执行指令的地址。
- `sstatus` 保存了 S 态的状态寄存器，包含了 `U` 和 `S` 的权限位。
- `sscratch` 保存了 `Kernel Stack` 的栈顶地址，用于在 `__alltraps` 中回到用户程序的 `Kernel Stack`

> Q3: L50-L56：为何跳过了 x2 和 x4？

A: `x2` 就是 `sp` 寄存器，指向了栈顶地址。这个我们在恢复完其他的寄存器之后才会设置

`x4` 寄存器我们的应用没有用到它，它的 · 是 `tp` 线程指针 (Thread Pointer)。

> Q4: L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

A: 将应用的 `kernel stack` 的栈顶地址保存在了 `sscratch`，`sp` 保存了 `user stack` 的栈顶地址，将 `kernel stack` 保存到 `sscratch` 的目的是为了在 `__alltraps` 获取应用 `kernel stack` 地址。

> Q5: __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

A: `sret` 指令

> Q6: L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

A: 和 Q4 的目的相反。

> Q7: 从 U 态进入 S 态是哪一条指令发生的？

A: 应用程序通过调用 `ecall` 指令。
