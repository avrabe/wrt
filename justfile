# Default recipe to run when just is called without arguments
default: build

# Variables for Sphinx documentation
sphinx_source := "docs/source"
sphinx_build_dir := "docs/_build"
sphinx_opts := "-E" # -E: rebuild all files, -a: write all files
sphinx_build := "sphinx-build"

# ----------------- Build Commands -----------------

# Build all crates and WAT files
build: build-wrt build-wrtd build-example build-adapter

# Build the core WRT library
build-wrt:
    cargo build -p wrt --all-features

# Build the WRT daemon
build-wrtd:
    cargo build -p wrtd --all-features

# Build the example module (debug mode)
build-example: setup-rust-targets
    cargo build -p example --target wasm32-wasip2

# Build the example module in release mode (optimized for size)
build-example-release: setup-rust-targets
    cargo build -p example --target wasm32-wasip2 --release

# Build the logging adapter component (debug mode)
build-adapter: setup-rust-targets
    cargo install cargo-component --locked || true
    cargo component build -p logging-adapter --target wasm32-wasip2

# Build the logging adapter component (release mode)
build-adapter-release: setup-rust-targets
    cargo component build -p logging-adapter --target wasm32-wasip2 --release

# ----------------- Setup Commands -----------------
setup-rust-targets:
    rustup target add wasm32-unknown-unknown wasm32-wasip1 wasm32-wasip2 || true

# ----------------- Coverage Commands -----------------
coverage:
    @echo "Generating quick code coverage via xtask..."
    cargo xtask coverage

coverage-comprehensive:
    @echo "Generating comprehensive code coverage (features, platforms, MCDC, Kani)..."
    cargo xtask coverage-comprehensive

generate-coverage-summary:
    @echo "Generating coverage summary for documentation..."
    cargo xtask generate-coverage-summary

generate-safety-summary:
    @echo "Generating safety verification summary for documentation..."
    cargo xtask generate-safety-summary

# ----------------- Formatting Commands -----------------
fmt:
    @echo "Formatting Rust code..."
    cargo fmt

fmt-check:
    @echo "Checking Rust code formatting (Daggerized)..."
    cargo xtask fmt-check

# ----------------- CI Tasks (Safety-Critical Rust Checklist) -----------------

# Consolidated Integrity Checks (Toolchain, File Presence, Headers)
ci-integrity-checks:
    @echo "CI: Running Daggerized integrity checks (toolchain, file presence, headers)..."
    cargo xtask ci-integrity-checks

# Consolidated Static Analysis
ci-static-analysis:
    @echo "CI: Running Daggerized static analysis pipeline..."
    cargo xtask ci-static-analysis

# Consolidated Advanced Tests (Kani, Miri, Coverage)
ci-advanced-tests:
    @echo "CI: Running Daggerized advanced tests pipeline (Kani, Miri, Coverage)..."
    cargo xtask ci-advanced-tests

# Rule 9: Documentation
ci-doc-check:
    @echo "CI: Running local strict documentation checks via xtask..."
    cargo xtask check-docs-strict

# General test suite execution for CI
ci-test:
    @echo "CI: Running all tests (Daggerized with feature configs)..."
    cargo xtask run-tests

# Safety verification for CI
ci-safety:
    @echo "CI: Running SCORE-inspired safety verification pipeline..."
    cargo xtask ci-safety --threshold 70.0 --fail-on-safety-issues

# Aggregate CI check - runs most critical checks including safety
ci-main: default ci-integrity-checks fmt-check ci-static-analysis ci-test ci-doc-check ci-safety

# Full CI suite - includes longer running checks
ci-full: ci-main ci-advanced-tests

# Comprehensive build matrix verification with architectural analysis
verify-build-matrix:
	@echo "Running comprehensive build matrix verification..."
	./scripts/verify-build-matrix.sh

# ----------------- Specific Test Runners & Dev Utilities -----------------

# Test wrtd with the example component (release mode)
# Additional arguments can be passed with e.g. `just test-wrtd-example "--fuel 100 --stats"`
test-wrtd-example *ARGS="--call example:hello/example#hello": setup-rust-targets build-example-release build-wrtd
    ./target/debug/wrtd {{ARGS}} ./target/wasm32-wasip2/release/example.wasm
    echo -n "Size of ./target/wasm32-wasip2/release/example.wasm: "
    @cargo xtask fs file-size ./target/wasm32-wasip2/release/example.wasm

# Test wrtd with help output (simple check)
test-wrtd-help: build-wrtd
    ./target/debug/wrtd --help

# ----------------- xtask Integration -----------------
# Delegate tasks to cargo-xtask for more complex operations
xtask *ARGS:
    cargo xtask {{ARGS}}

# ----------------- Clean Commands -----------------
clean:
    cargo clean
    rm -rf target/llvm-cov/
    rm -f lcov.info coverage.json docs/baseline_coverage.md
    rm -rf "{{sphinx_build_dir}}"

# ----------------- Zephyr related tasks (Preserve these) -----------------
ZEPHYR_SDK_VERSION := "0.16.5-1"
ZEPHYR_WEST_VERSION := "1.2.0"
ZEPHYR_PROJECT_DIR := ".zephyrproject/zephyr"

zephyr-setup-sdk:
    @echo "Setting up Zephyr SDK..."
    # (SDK location needs to be configured, e.g. via environment variable or direct path)
    # export ZEPHYR_SDK_INSTALL_DIR=/path/to/zephyr-sdk-{{ZEPHYR_SDK_VERSION}}
    # west zephyr-export # If west manages the SDK

zephyr-setup-venv:
    @echo "Setting up Zephyr Python virtual environment..."
    python3 -m venv .zephyr-venv
    source .zephyr-venv/bin/activate && pip install west=={{ZEPHYR_WEST_VERSION}} wheel
    source .zephyr-venv/bin/activate && pip install -r {{ZEPHYR_PROJECT_DIR}}/scripts/requirements.txt

zephyr-init:
    @echo "Initializing/updating Zephyr workspace (west init/update)..."
    # west init -l path/to/your/manifest # If not already initialized
    source .zephyr-venv/bin/activate # Ensure venv is active
    west update

zephyr-build APP_NAME="hello_world" BOARD="native_posix":
    @echo "Building Zephyr application: {{APP_NAME}} for board: {{BOARD}}..."
    source .zephyr-venv/bin/activate
    west build -b {{BOARD}} {{ZEPHYR_PROJECT_DIR}}/samples/basic/{{APP_NAME}} # Adjusted path for common sample

zephyr-run APP_NAME="hello_world" BOARD="native_posix": # Added BOARD to run for clarity
    @echo "Running Zephyr application: {{APP_NAME}} on board: {{BOARD}}..."
    source .zephyr-venv/bin/activate
    west build -b {{BOARD}} -t run {{ZEPHYR_PROJECT_DIR}}/samples/basic/{{APP_NAME}}/build # Build dir path might vary

# Add other Zephyr-specific tasks here as needed
# Example: zephyr-flash, zephyr-debug, etc.

# ----------------- SCORE-Inspired Safety Verification -----------------
# All safety verification commands are implemented in xtask for proper integration

# Run SCORE-inspired safety verification
verify-safety:
    @echo "🔍 Running SCORE-inspired safety verification..."
    cargo xtask verify-safety

# Check requirements traceability
check-requirements:
    @echo "📋 Checking requirements traceability..."
    cargo xtask check-requirements

# Initialize requirements file from template
init-requirements:
    @echo "📋 Creating sample requirements.toml..."
    cargo xtask init-requirements

# Generate safety verification report (supports --format json/html)
safety-report:
    @echo "📊 Generating safety verification report..."
    cargo xtask safety-report

# Run requirements verification with file checking (supports --detailed)
verify-requirements:
    @echo "🔍 Verifying requirements implementation..."
    cargo xtask verify-requirements

# Safety dashboard - comprehensive safety status
safety-dashboard:
    @echo "🛡️  WRT Safety Dashboard"
    @echo "========================"
    cargo xtask safety-dashboard

# Safety verification examples with advanced options:
# cargo xtask verify-safety --format json --output safety.json
# cargo xtask verify-requirements --detailed --requirements-file custom.toml
# cargo xtask safety-report --format html --output report.html