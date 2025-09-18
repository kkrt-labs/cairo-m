PANDOC ?= pandoc
DOCS_DIR := docs
TEXCACHE := $(DOCS_DIR)/.texcache

.PHONY: design-pdf
design-pdf:
	@mkdir -p $(TEXCACHE)
	@cd $(DOCS_DIR) && \
		TEXMFCACHE=$$(pwd)/.texcache TEXMFVAR=$$(pwd)/.texcache \
		$(PANDOC) --defaults pandoc.yaml -o design.pdf && \
		echo "Wrote $(DOCS_DIR)/design.pdf"

