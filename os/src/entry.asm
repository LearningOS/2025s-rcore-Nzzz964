    .section .text.entry
    .globl _start
_start:
    # load address，将 boot_stack_top 加载到 sp 寄存器中
    la sp, boot_stack_top
    call rust_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top: