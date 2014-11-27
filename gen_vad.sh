#!/bin/sh
set -x
set -e
cd vad
autoreconf -vfi
./configure --host=$TARGET
make clean
make
mv libvad.a $OUT_DIR
cd -
