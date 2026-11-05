PLUGIN := se.fredrik.codexusage.sdPlugin
.PHONY: check stage package clean
check:
	cargo fmt --check
	cargo test --locked
	cargo clippy --locked --all-targets -- -D warnings
stage:
	cargo build --locked --release
	mkdir -p dist/$(PLUGIN)
	cp assets/* dist/$(PLUGIN)/
	cp LICENSE dist/$(PLUGIN)/
	cp target/release/opendeck-codex-usage dist/$(PLUGIN)/
package: stage
	cd dist && zip -qr codex-usage-linux-x86_64.zip $(PLUGIN)
clean:
	cargo clean
	rm -rf dist
