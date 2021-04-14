#!/bin/sh

TOP=$(git rev-parse --show-toplevel)
cd ${TOP}
# RATE=115200
RATE=921600
cat kern/build/kernel.bin | lib/ttywrite/target/debug/ttywrite /dev/ttyUSB0 --baud ${RATE} --timeout 2
picocom -b ${RATE} /dev/ttyUSB0
