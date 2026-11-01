# Comandi rapidi di photo-cat (GNU Make; su Windows funziona sia da cmd che da Git Bash).
#   make dev       avvio locale in debug (Vite + app con ricarica)
#   make build     test di build come la CI: setup NSIS su Windows, .deb e .tar.gz su Linux
#   make test      test completi in locale (Rust + UI)
#   make test-linux  passi della CI Linux in Docker (da Windows o macOS)
#   make release   tag v<versione> e push: la GitHub Action crea la release

ifeq ($(OS),Windows_NT)
NPM := npm.cmd
BUILD_HELP := setup NSIS in target/release/bundle/nsis
else
NPM := npm
BUILD_HELP := .deb e .tar.gz in target/release/dist
endif
UI := app/ui
VERSION := $(shell node -p "require('./app/src-tauri/tauri.conf.json').version")

.PHONY: help install dev run build test test-linux release

help:
	@echo "make dev | run   - avvio locale (debug)"
	@echo "make build       - test di build: $(BUILD_HELP)"
	@echo "make test        - fmt, clippy, test Rust e UI"
	@echo "make test-linux  - CI Linux in Docker: fmt, clippy, test, pacchetti"
	@echo "make release     - crea e pubblica il tag v$(VERSION) (release automatica su GitHub)"

# Reinstalla le dipendenze UI solo se package-lock.json è cambiato (npm aggiorna lo stamp).
DEPS := $(UI)/node_modules/.package-lock.json

install: $(DEPS)

$(DEPS): $(UI)/package-lock.json
	$(NPM) ci --prefix $(UI)

dev: install
	$(NPM) --prefix $(UI) run tauri -- dev

run: dev

build: install
ifeq ($(OS),Windows_NT)
	$(NPM) --prefix $(UI) run tauri -- build --bundles nsis
else
	scripts/package-linux.sh
endif

test: install
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	$(NPM) --prefix $(UI) run typecheck
	$(NPM) --prefix $(UI) run lint
	$(NPM) --prefix $(UI) test

# CI Linux in Docker (Ubuntu 22.04), per verificare da Windows o macOS: vedi scripts/test-linux-docker.sh.
test-linux:
	bash scripts/test-linux-docker.sh

# La versione viene da tauri.conf.json (deve coincidere con Cargo.toml: la pipeline lo verifica).
release:
	git diff --quiet HEAD
	git fetch --tags origin
	git tag -a v$(VERSION) -m "photo-cat v$(VERSION)"
	git push origin HEAD
	git push origin v$(VERSION)
	@echo "Tag v$(VERSION) pubblicato. Release in preparazione: https://github.com/federicocunico/photo-cat/releases"
