all:
	cargo +nightly build --target=wasm32-unknown-unknown --release
	wasm-gc target/wasm32-unknown-unknown/release/fs_dither_wasm.wasm target/wasm32-unknown-unknown/release/fs_dither.wasm
	cp index.html target/wasm32-unknown-unknown/release/index.html
