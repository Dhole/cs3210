.section .text.init

.global _start

_start:
    // read cpu affinity, start core 0, halt rest
    mrs     x1, mpidr_el1
    and     x1, x1, #3
    cbz     x1, 2f

1:
    // core affinity != 0, halt it
    wfe
    b       1b

2:
    // set the stack to start before our boot code
    adr     x1, _start
    mov     sp, x1


//        adrp    x9, _start
//        mov     w8, #0xc6c0                     // #50880
//        movk    w8, #0x2d, lsl #16
//        mov     w13, #0x40000                   // #262144
//        mov     w10, #0x10000                   // #65536
//        adrp    x11, _start
//        ldr     x12, [x9, #168]
//        adrp    x9, _start
//        str     w13, [x12]
// c:     ldr     x13, [x9, #176]
//        mov     w12, w8
//        str     w10, [x13]
// a:     subs    w12, w12, #0x1
//        nop
//        b.ne    a   // b.any
//        ldr     x13, [x11, #184]
//        mov     w12, w8
//        str     w10, [x13]
// b:     subs    w12, w12, #0x1
//        nop
//        b.ne    b   // b.any
//        b       c

    // jump to kinit, which shouldn't return. halt if it does
    bl      kinit
    b       1b
