#!/bin/bash

set -euo pipefail

cd /src/iot-edge-spiffe-server

. ../ci/install-build-deps.sh

make -f /src/Makefile SRC=data codecov
mv coverage /src

cd /src/coverage
apt install lcov -y
lcov --add-tracefile lcov.info