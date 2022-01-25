#!/bin/bash

set -euo pipefail

cd /src/iot-edge-spiffe-server

. ../ci/install-build-deps.sh

make -f /src/Makefile SRC=data V=1 test-release