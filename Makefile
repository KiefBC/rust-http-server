.PHONY: ci

ci:
	RUSTC_WRAPPER= cargo test
