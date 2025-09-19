PANDOC ?= pandoc
DOCS_DIR := docs
TEXCACHE := $(DOCS_DIR)/.texcache
DATE := $(shell date +%Y-%m-%d)
AUTHOR := Clément Walter <clement@kakarot.org>

.PHONY: design-pdf
design-pdf:
	@mkdir -p $(TEXCACHE)
	@cd $(DOCS_DIR) && \
		TEXMFCACHE=$$(pwd)/.texcache TEXMFVAR=$$(pwd)/.texcache \
		$(PANDOC) --defaults pandoc.yaml -M author="$(AUTHOR)" -M date="$(DATE)" -o design.pdf && \
		echo "Wrote $(DOCS_DIR)/design.pdf"
