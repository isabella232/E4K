#!/bin/bash

set -euo pipefail

cd /src/identities

. ../ci/install-build-deps.sh

make -f /src/Makefile SRC=data codecov

cd coverage
apt install lcov -y
lcov --add-tracefile lcov.info