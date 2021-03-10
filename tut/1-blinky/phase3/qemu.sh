#!/bin/sh

# TOP=$(git rev-parse --show-toplevel)
# $TOP/bin/qemu-system-aarch64 \
qemu-system-aarch64 \
    -s -S \
    -nographic \
    -M raspi3 \
    -serial null -serial mon:stdio \
    -kernel \
    "$@"
