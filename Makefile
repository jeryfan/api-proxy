# API 代理 — 开发与构建快捷命令

APP_NAME       := API 代理
MACOS_APP      := src-tauri/target/release/bundle/macos/$(APP_NAME).app
MACOS_DEST     := /Applications/$(APP_NAME).app
LINUX_DEB_GLOB := src-tauri/target/release/bundle/deb/*.deb

.PHONY: help dev build install clean clean-all

help:
	@echo "API 代理 — 可用命令"
	@echo ""
	@echo "  make dev        启动开发模式"
	@echo "  make build      构建桌面安装包"
	@echo "  make install    构建并安装到系统"
	@echo "  make clean      清理构建产物"
	@echo "  make clean-all  清理产物 + node_modules"

dev:
	pnpm tauri dev

build:
	pnpm tauri build

install: build
	@uname_s=$$(uname); \
	case "$$uname_s" in \
		Darwin) \
			echo "→ 安装到 $(MACOS_DEST)"; \
			rm -rf "$(MACOS_DEST)"; \
			cp -R "$(MACOS_APP)" /Applications/; \
			echo "✓ 已安装。可在 Launchpad 或 Applications 文件夹打开。"; \
			;; \
		Linux) \
			echo "→ 安装 .deb 包（需要 sudo）"; \
			sudo dpkg -i $(LINUX_DEB_GLOB); \
			;; \
		*) \
			echo "✗ 不支持的系统：$$uname_s"; \
			exit 1; \
			;; \
	esac

clean:
	rm -rf dist
	cd src-tauri && cargo clean

clean-all: clean
	rm -rf node_modules
