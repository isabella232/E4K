#!/bin/bash

set -euo pipefail

cd /src

. ./ci/install-build-deps.sh

make -f /src/Makefile SRC=data V=1 test-release