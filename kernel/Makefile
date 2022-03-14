

all:
	cargo build --release
	rust-objcopy --strip-all -O binary target/aarch64-unknown-none/release/ruxpin ruxpin.img

