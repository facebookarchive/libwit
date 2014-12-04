#!/bin/sh
set -x
set -e
cd fake
gcc -g -c fake.c
ar crus libfake.a fake.o
rm fake.o
mv libfake.a $OUT_DIR
cd -

