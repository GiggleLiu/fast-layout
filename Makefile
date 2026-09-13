.PHONY: all pkgroot test rust-test manual plugin install clean bench bench-typst package check-package

export TYPST_PACKAGE_PATH := $(CURDIR)/_pkgroot

all: test

pkgroot:
	@rm -rf _pkgroot/preview/fast-layout
	@mkdir -p _pkgroot/preview/fast-layout
	@ln -sfn $(CURDIR)/fast-layout _pkgroot/preview/fast-layout/0.1.0

rust-test:
	cargo test --locked -p fast-layout-engine

test: pkgroot rust-test
	@$(MAKE) -C fast-layout test

manual: pkgroot
	@$(MAKE) -C fast-layout manual

plugin:
	@$(MAKE) -C fast-layout plugin

bench:
	cargo bench -p fast-layout-engine --bench layouts

bench-typst: pkgroot plugin
	python3 scripts/bench_warm.py $(BENCH_TYPST_ARGS)

PACKAGE_DIR := $(CURDIR)/_dist/preview/fast-layout/0.1.0

package: manual
	rm -rf "$(PACKAGE_DIR)"
	mkdir -p "$(PACKAGE_DIR)"
	cp fast-layout/typst.toml fast-layout/lib.typ fast-layout/README.md \
	  fast-layout/LICENSE fast-layout/NETWORKLAYOUT-LICENSE.md \
	  fast-layout/THIRD_PARTY-NOTICES.md fast-layout/THIRD-PARTY-LICENSES.txt \
	  fast-layout/manual.pdf "$(PACKAGE_DIR)/"
	cp -R fast-layout/src fast-layout/plugin fast-layout/images "$(PACKAGE_DIR)/"

check-package: package
	docker run --rm -v "$(PACKAGE_DIR):/data" ghcr.io/typst/package-check check
	TYPST_PACKAGE_PATH="$(CURDIR)/_dist" python3 scripts/check_examples.py

TYPST_DATA_DIR := $(if $(filter Darwin,$(shell uname -s)),$(HOME)/Library/Application Support,$(if $(XDG_DATA_HOME),$(XDG_DATA_HOME),$(HOME)/.local/share))

install:
	@mkdir -p "$(TYPST_DATA_DIR)/typst/packages/preview/fast-layout"
	@ln -sfn "$(CURDIR)/fast-layout" "$(TYPST_DATA_DIR)/typst/packages/preview/fast-layout/0.1.0"

clean:
	rm -rf _pkgroot _dist
	@$(MAKE) -C fast-layout clean
