#!/bin/sh
cd vad
./configure
make
mv libvad.a ../
cd -

