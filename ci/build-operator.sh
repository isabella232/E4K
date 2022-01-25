#!/bin/sh

set -euo pipefail

cd /src/control-plane

. ../ci/install-build-deps.sh

make -f /src/Makefile V=1