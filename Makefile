CARGO = cargo

# 0 => false, _ => true
V = 0

# 0 => false, _ => true
RELEASE = 0

# '' => amd64, 'arm32v7' => arm32v7, 'aarch64' => aarch64
ARCH =

# 'control' => control-plane , 'data' => data-plane
SRC=

ifeq ($(V), 0)
	CARGO_VERBOSE = --quiet
else
	CARGO_VERBOSE = --verbose
endif

ifeq ($(RELEASE), 0)
	CARGO_PROFILE =
	CARGO_PROFILE_DIRECTORY = debug
else
	CARGO_PROFILE = --release
	CARGO_PROFILE_DIRECTORY = release
endif

ifeq ($(SRC), data)
	ifeq ($(ARCH), arm32v7)
		CARGO_TARGET = armv7-unknown-linux-gnueabihf
	else ifeq ($(ARCH), aarch64)
		CARGO_TARGET = aarch64-unknown-linux-gnu
	else
		CARGO_TARGET = x86_64-unknown-linux-gnu
	endif
endif

ifdef CARGO_TARGET
	CARGO_TARGET_ARG = --target $(CARGO_TARGET)
endif

ifeq ($(SRC), data)
	PACKAGE = -p serverd -p agentd -p identity-manager
	TEST_FEATURES = --features 'tests'
endif

#ifndef IMAGE_REPOSITORY
#	IMAGE_REPOSITORY = dmqtt-operator
#endif

#ifndef IMAGE_TAG
#	IMAGE_TAG = latest
#endif

#ifeq ($(ARCH), arm32v7)
#	DOCKERFILE_FOLDER = arm32
#else ifeq ($(ARCH), aarch64)
#	DOCKERFILE_FOLDER = arm64
#else
#	DOCKERFILE_FOLDER = amd64
#endif

# Some of the targets use bash-isms like `set -o pipefail`
SHELL := /bin/bash


.PHONY: clean codecov default test test-release


default:
	$(CARGO) build \
		$(PACKAGE) \
		$(CARGO_TARGET_ARG) $(CARGO_PROFILE) $(CARGO_VERBOSE)

clean:
	$(CARGO) clean $(CARGO_VERBOSE)

codecov:
	$(CARGO) tarpaulin --all $(TEST_FEATURES) \
		$(CARGO_TARGET_ARG) --verbose \
		--out Lcov --output-dir ./coverage

test: default
test:
	set -eo pipefail; \
	$(CARGO) test --all $(TEST_FEATURES) \
		$(CARGO_TARGET_ARG) $(CARGO_VERBOSE) 2>&1 | \
		grep -v 'running 0 tests' | grep -v '0 passed; 0 failed' | (grep '.' || :); \
	$(CARGO) clippy --all $(TEST_FEATURES) \
		$(CARGO_TARGET_ARG) $(CARGO_VERBOSE) -- $(CLIPPY_FLAGS); \
	$(CARGO) clippy --all --tests $(TEST_FEATURES) \
		$(CARGO_TARGET_ARG) $(CARGO_VERBOSE) -- $(CLIPPY_FLAGS); \

	find . '(' -name 'target' -o -wholename './buffer-pool/src/tests' ')' -prune -o -type f -name '*.rs' -print | \
		grep -E '/(build|lib|main|tests/[^/]+)\.rs$$' | \
		while read -r f; do \
			if ! grep -Eq '^#!\[deny\(rust_2018_idioms\)\]$$' "$$f"; then \
				echo "missing #![deny(rust_2018_idioms)] in $$f" >&2; \
				exit 1; \
			fi; \
			if ! grep -Eq '^#!\[warn\(clippy::all, clippy::pedantic\)\]$$' "$$f"; then \
				echo "missing #![warn(clippy::all, clippy::pedantic)] in $$f" >&2; \
				exit 1; \
			fi; \
		done

	find . -name 'Makefile' -or -name '*.c' -or -name '*.md' -or -name '*.rs' -or -name '*.toml' -or -name '*.txt' | \
		grep -v '^\./target/' | \
		grep -v '\.generated\.rs$$' | \
		grep -v 'Config.toml' | \
		while read -r f; do \
			if [[ -s "$$f" && "$$(tail -c 1 "$$f" | wc -l)" -eq '0' ]]; then \
				echo "missing newline at end of $$f" >&2; \
				exit 1; \
			fi; \
		done

	find . -name '*.c' -or -name '*.rs' | \
		grep -v '^\./target/' | \
		grep -v '\.generated\.rs$$' | \
		while read -r f; do \
			if ! (head -n1 "$$f" | grep -q 'Copyright (c) Microsoft. All rights reserved.'); then \
				echo "missing copyright header in $$f" >&2; \
				exit 1; \
			fi; \
		done


test-release: CLIPPY_FLAGS = -D warnings -D clippy::all -D clippy::pedantic
test-release: test
test-release:
	$(CARGO) fmt --all -- --check

