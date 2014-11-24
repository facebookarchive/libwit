#!/bin/sh
set -x
set -e
cd vad
autoreconf -vfi
./configure
make clean
make
mv libvad.a $OUT_DIR
ln -s $OUT_DIR/libvad.a .
cd -

