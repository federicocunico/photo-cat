# Comandi rapidi di photo-cat (GNU Make; su Windows funziona sia da cmd che da Git Bash).
#   make dev       avvio locale in debug (Vite + app con ricarica)
#   make build     test di build: installer NSIS come la CI
#   make test      test completi in locale (Rust + UI)
#   make release   tag v<versione> e push: la GitHub Action crea la release

ifeq ($(OS),Windows_NT)
NPM := npm.cmd
else
NPM := npm
endif
UI := app/ui
VERSION := $(shell node -p "require('./app/src-tauri/tauri.conf.json').version")

.PHONY: help install dev run build test release

help:
	@echo "make dev | run   - avvio locale (debug)"
	@echo "make build       - test di build: setup NSIS in target/release/bundle/nsis"
	@echo "make test        - fmt, clippy, test Rust e UI"
	@echo "make release     - crea e pubblica il tag v$(VERSION) (release automatica su GitHub)"

install:
	$(NPM) ci --prefix $(UI)

dev: install
	$(NPM) --prefix $(UI) run tauri -- dev

run: dev

build: install
	$(NPM) --prefix $(UI) run tauri -- build --bundles nsis

test: install
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	$(NPM) --prefix $(UI) run typecheck
	$(NPM) --prefix $(UI) run lint
	$(NPM) --prefix $(UI) test

# La versione viene da tauri.conf.json (deve coincidere con Cargo.toml: la pipeline lo verifica).
release:
	git diff --quiet HEAD
	git fetch --tags origin
	git tag -a v$(VERSION) -m "photo-cat v$(VERSION)"
	git push origin HEAD
	git push origin v$(VERSION)
	@echo "Tag v$(VERSION) pubblicato: la release sarà su https://github.com/federicocunico/photo-cat/releases"
