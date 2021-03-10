aarch64-linux-gnu-gdb build/blinky.elf

target remote localhost:1234
handle SIGTRAP  nostop noprint

