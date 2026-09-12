.PHONY: all test examples plugin clean manual

TYPST = typst compile --root .
TESTS := $(wildcard tests/test-*.typ)
ERROR_TESTS := $(wildcard tests/errors/*.typ)
EXAMPLES := $(wildcard examples/*.typ)

all: test

manual:
	$(TYPST) manual.typ manual.pdf

test: plugin manual
	@for t in $(TESTS); do \
	  echo "== $$t"; \
	  $(TYPST) $$t $${t%.typ}.pdf || exit 1; \
	done
	@for t in $(ERROR_TESTS); do \
	  echo "== $$t (expected error)"; \
	  expected=$$(sed -n 's|^// expected: ||p' $$t); \
	  test -n "$$expected" || { echo "missing // expected: marker in $$t"; exit 1; }; \
	  out=$$($(TYPST) $$t /tmp/fast-layout-expected-error.pdf 2>&1); status=$$?; \
	  test $$status -ne 0 || { echo "expected compilation to fail"; exit 1; }; \
	  echo "$$out" | grep -F -- "$$expected" >/dev/null || { echo "$$out"; exit 1; }; \
	done

examples:
	@for f in $(EXAMPLES); do \
	  echo "== $$f"; \
	  $(TYPST) $$f $${f%.typ}.pdf || exit 1; \
	done

WASM_OUT = plugin/fast_layout_engine.wasm
RUST_SYSROOT = $(shell rustc --print sysroot)
RUST_COMMIT = $(shell rustc -vV | sed -n 's/^commit-hash: //p')
CARGO_HOME_DIR = $(shell echo "$${CARGO_HOME:-$$HOME/.cargo}")
WORKSPACE_ROOT = $(abspath ..)
REPRO_RUSTFLAGS = --remap-path-prefix=$(CARGO_HOME_DIR)=/cargo --remap-path-prefix=$(WORKSPACE_ROOT)=/build --remap-path-prefix=$(RUST_SYSROOT)/lib/rustlib/src/rust=/rustc/$(RUST_COMMIT)

plugin:
	RUSTFLAGS="$(REPRO_RUSTFLAGS)" cargo build --manifest-path ../Cargo.toml -p fast-layout-engine --release --target wasm32-unknown-unknown
	@mkdir -p plugin
	cp ../target/wasm32-unknown-unknown/release/fast_layout_engine.wasm $(WASM_OUT)

clean:
	rm -f tests/*.pdf tests/errors/*.pdf examples/*.pdf manual.pdf
