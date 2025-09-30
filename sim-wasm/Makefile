WASM_PACK ?= wasm-pack
OUT_DIR ?= ~/code/ravelin-game/src/lib/pkg
WASM_TARGET ?= web

.PHONY: build release clean

build:
	$(WASM_PACK) build --target $(WASM_TARGET) --dev --out-dir $(OUT_DIR)

release:
	$(WASM_PACK) build --target $(WASM_TARGET) --release --out-dir $(OUT_DIR)

clean:
	rm -rf $(OUT_DIR)
