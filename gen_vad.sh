#!/bin/sh
set -x
cd vad
aclocal
automake --add-missing
autoconf
./configure
make
mv libvad.a ../
cd -

