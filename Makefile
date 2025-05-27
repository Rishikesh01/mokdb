NEXTEST_VERSION := 0.9.74

.PHONY: test
test: nextest-install
	RUSTFLAGS="-Awarnings" cargo nextest run --no-fail-fast

.PHONY: test-release
test-release: nextest-install
	RUSTFLAGS="-Awarnings" cargo nextest run --release --no-fail-fast

.PHONY: nextest-install
nextest-install:
	@installed_version=$$(cargo nextest --version 2>/dev/null | cut -d' ' -f2); \
	if [ "$$installed_version" != "$(NEXTEST_VERSION)" ]; then \
		echo "Installing cargo-nextest $(NEXTEST_VERSION)..."; \
		cargo install cargo-nextest --version $(NEXTEST_VERSION) --locked --force; \
	else \
		echo "cargo-nextest $(NEXTEST_VERSION) already installed."; \
	fi

.PHONY: lint
lint:
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: lint-tests
lint-tests:
	cargo clippy --tests -- -D warnings

.PHONY: fmt
fmt:
	cargo fmt -- --check
