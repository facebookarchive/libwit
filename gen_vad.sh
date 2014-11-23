#!/bin/sh
set -x
set -e
cd vad
autoreconf -vfi
./configure
make clean
make
ln -s libvad.a $OUT_DIR
cd -
