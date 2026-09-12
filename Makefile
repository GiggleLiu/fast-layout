.PHONY: all pkgroot test rust-test manual plugin install clean bench bench-typst bench-incremental

export TYPST_PACKAGE_PATH := $(CURDIR)/_pkgroot

all: test

pkgroot:
	@rm -rf _pkgroot/preview/fast-layout
	@mkdir -p _pkgroot/preview/fast-layout
	@ln -sfn $(CURDIR)/fast-layout _pkgroot/preview/fast-layout/0.1.0

rust-test:
	cargo test -p fast-layout-engine

test: pkgroot rust-test
	@$(MAKE) -C fast-layout test

manual: pkgroot
	@$(MAKE) -C fast-layout manual

plugin:
	@$(MAKE) -C fast-layout plugin

bench:
	cargo bench -p fast-layout-engine --bench layouts

bench-typst: pkgroot plugin
	python3 scripts/bench_typst.py $(BENCH_TYPST_ARGS)

bench-incremental: pkgroot plugin
	python3 scripts/bench_incremental.py $(BENCH_INCREMENTAL_ARGS)

TYPST_DATA_DIR := $(if $(filter Darwin,$(shell uname -s)),$(HOME)/Library/Application Support,$(if $(XDG_DATA_HOME),$(XDG_DATA_HOME),$(HOME)/.local/share))

install:
	@mkdir -p "$(TYPST_DATA_DIR)/typst/packages/preview/fast-layout"
	@ln -sfn "$(CURDIR)/fast-layout" "$(TYPST_DATA_DIR)/typst/packages/preview/fast-layout/0.1.0"

clean:
	rm -rf _pkgroot
	@$(MAKE) -C fast-layout clean
