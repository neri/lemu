.PHONY: all clean test test-native test-common test-harvard test-flat test-matrix test-wasm test-tools update doc

CARGO ?= cargo
PYTHON ?= python3
NODE ?= node

all:

clean:
	cargo clean

update:
	cargo update

test: test-native

test-native:
	$(CARGO) test --workspace
	$(CARGO) test -p plum --example frequency_bench guest_bytes::tests
	$(PYTHON) scripts/test/plum.py --cargo "$(CARGO)" --matrix

test-common test-harvard test-flat:
	$(PYTHON) scripts/test/plum.py --cargo "$(CARGO)" --suite $(@:test-%=%)

test-matrix:
	$(PYTHON) scripts/test/plum.py --cargo "$(CARGO)" --matrix

test-wasm:
	$(PYTHON) scripts/test/wasm.py --cargo "$(CARGO)" --node "$(NODE)"

test-tools:
	$(PYTHON) scripts/test/test-plum-frequency.py
	$(PYTHON) scripts/test/corpus.py

doc:
	cargo doc --no-deps
