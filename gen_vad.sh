#!/bin/sh
set -x
cd vad
autoreconf -vfi
./configure
make
mv libvad.a ../
cd -

