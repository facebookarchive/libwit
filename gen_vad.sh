#!/bin/sh
set -x
cd vad
autoreconf -vfi
./configure
make clean
make
mv libvad.a ../
cd -

