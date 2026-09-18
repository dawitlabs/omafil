PREFIX ?= $(HOME)/.local
BIN := $(PREFIX)/bin/omafil
APPS := $(PREFIX)/share/applications
ICONS := $(PREFIX)/share/icons/hicolor

.PHONY: build install uninstall default restore-default test

build:
	cd src && bun install --frozen-lockfile
	cd src-tauri && cargo tauri build --no-bundle

install: build
	install -Dm755 src-tauri/target/release/omafil $(BIN)
	install -Dm644 packaging/omafil.desktop $(APPS)/omafil.desktop
	install -Dm644 src-tauri/icons/128x128.png $(ICONS)/128x128/apps/omafil.png
	install -Dm644 src-tauri/icons/32x32.png $(ICONS)/32x32/apps/omafil.png
	update-desktop-database $(APPS) 2>/dev/null || true

default:
	"$(BIN)" --make-default

restore-default:
	"$(BIN)" --restore-default

uninstall:
	@if test -f "$${XDG_CONFIG_HOME:-$$HOME/.config}/omafil/desktop-integration.json"; then "$(BIN)" --restore-default; fi
	rm -f $(BIN) $(APPS)/omafil.desktop $(ICONS)/128x128/apps/omafil.png $(ICONS)/32x32/apps/omafil.png
	update-desktop-database $(APPS) 2>/dev/null || true

test:
	cd core && cargo test -q
	cd src-tauri && cargo test -q
	cd src && bun test && bun run check
