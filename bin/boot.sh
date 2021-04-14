#!/bin/sh

TOP=$(git rev-parse --show-toplevel)
cd ${TOP}
cat kern/build/kernel.bin | lib/ttywrite/target/debug/ttywrite /dev/ttyUSB0 --baud 115200 --timeout 2
picocom -b 115200 /dev/ttyUSB0
