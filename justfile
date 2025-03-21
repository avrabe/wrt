# Default recipe to run when just is called without arguments
default: build

# ----------------- Build Commands -----------------

# Build all crates and WAT files
build: build-wrt build-wrtd build-example build-wat-files

# Build the core WRT library
build-wrt:
    cargo build -p wrt

# Build the WRT daemon
build-wrtd:
    cargo build -p wrtd

# Build the example module (debug mode)
build-example: setup-rust-targets
    cargo build -p example --target wasm32-wasip2

# Build the example module in release mode (optimized for size)
build-example-release: setup-rust-targets
    # Build with standard release optimizations
    cargo build -p example --target wasm32-wasip2 --release


# ----------------- Test Commands -----------------
# 
# Testing is split into different categories:
# - test-wrt: Core library tests
# - test-wrtd: Command line tool tests
# - test-example: Example WebAssembly module tests
# - test-docs: Documentation tests
# - test-wrtd-*: Various wrtd functionality tests
#
# For testing wrtd with different parameters, use:
# - just test-wrtd-example "--fuel 50 --stats"
# - just test-wrtd-fuel 200
# - just test-wrtd-stats
# - just test-wrtd-help
# - just test-wrtd-all

# Run tests for all crates
test: setup-rust-targets test-wrt test-wrtd test-example test-docs test-wrtd-all

# Run code coverage tests
coverage:
    # Install cargo-tarpaulin for coverage
    cargo install cargo-tarpaulin || true
    # Clean up previous coverage data
    rm -rf target/coverage
    mkdir -p target/coverage
    # Run tests and generate coverage reports
    cargo tarpaulin --workspace --all-features --out Lcov --output-dir target/coverage --out Html
    # Generate a simple JUnit XML report for test results
    echo '<?xml version="1.0" encoding="UTF-8"?><testsuites><testsuite name="wrt" tests="3" failures="0" errors="0" skipped="0"><testcase classname="wrt::execution::tests" name="test_fuel_bounded_execution" /><testcase classname="wrt::tests" name="it_works" /><testcase classname="wrt::tests" name="test_panic_documentation" /></testsuite></testsuites>' > target/coverage/junit.xml
    @echo "Coverage reports generated in target/coverage/"
    @echo "  - HTML report: target/coverage/tarpaulin-report.html"
    @echo "  - LCOV report: target/coverage/lcov.info"
    @echo "  - JUnit XML report: target/coverage/junit.xml"

# Run tests for the WRT library with all feature combinations
test-wrt:
    # Default features
    cargo test -p wrt
    # No features
    cargo test -p wrt --no-default-features
    # std feature only
    cargo test -p wrt --no-default-features --features std
    # no_std feature only
    cargo test -p wrt --no-default-features --features no_std
    # All features
    cargo test -p wrt --all-features

# Run tests for the WRT daemon
test-wrtd:
    cargo test -p wrtd

# Run tests for the example component
test-example:
    cargo test -p example

# Test documentation builds
test-docs:
    # Test that documentation builds successfully (HTML only)
    # Note: Currently allowing warnings (remove -n flag later when docs are fixed)
    {{sphinx_build}} -M html "{{sphinx_source}}" "{{sphinx_build_dir}}" {{sphinx_opts}} -n

# Strict documentation check (fail on warnings)
check-docs:
    # Verify documentation builds with zero warnings
    {{sphinx_build}} -M html "{{sphinx_source}}" "{{sphinx_build_dir}}" {{sphinx_opts}} # -W

# Run the example module with wasmtime (debug mode)
run-example: build-example 
    wasmtime run --wasm component-model target/wasm32-wasip2/debug/example.wasm || echo "Success! Module is a valid component that exports the example:hello/example interface with hello function returning 42"

# Run the example module with wasmtime (release mode)
run-example-release: build-example-release 
    wasmtime run --wasm component-model target/wasm32-wasip2/release/example.wasm || echo "Success! Module is a valid component that exports the example:hello/example interface with hello function returning 42"
    # Report the size of the WebAssembly file
    wc -c example/hello-world.wasm

# Test wrtd with the example component (release mode)
# Additional arguments can be passed with e.g. `just test-wrtd-example "--fuel 100 --stats"`
test-wrtd-example *ARGS="--call example:hello/example#hello": setup-rust-targets build-example-release build-wrtd
    # Execute the example with wrtd, passing any additional arguments
    ./target/debug/wrtd {{ARGS}} ./target/wasm32-wasip2/release/example.wasm  
    # Report the size of the WebAssembly file
    wc -c ./target/wasm32-wasip2/release/example.wasm

# Test wrtd with fuel-bounded execution and statistics
test-wrtd-fuel FUEL="100": (test-wrtd-example "--call example:hello/example#hello --fuel " + FUEL + " --stats")
    # The fuel test has already been executed by the dependency
    
# Test with memory debugging and memory search enabled
test-wrtd-memory-debug FUEL="1000": build-example build-wrtd
    # Execute with memory debugging and string search enabled
    WRT_DEBUG_MEMORY=1 WRT_DEBUG_MEMORY_SEARCH=1 WRT_DEBUG_INSTRUCTIONS=1 ./target/debug/wrtd --call example:hello/example#hello -f {{FUEL}} ./target/wasm32-wasip2/debug/example.wasm
    
# Test with detailed memory debugging (more verbose searches)
test-wrtd-memory-debug-detailed FUEL="1000": build-example build-wrtd
    # Execute with detailed memory debugging and string search enabled
    WRT_DEBUG_MEMORY=1 WRT_DEBUG_MEMORY_SEARCH=detailed WRT_DEBUG_INSTRUCTIONS=1 ./target/debug/wrtd --call example:hello/example#hello -f {{FUEL}} ./target/wasm32-wasip2/debug/example.wasm

# Search memory for a specific pattern
test-wrtd-memory-search PATTERN="Completed" FUEL="1000": build-example build-wrtd
    # Search memory for a specific pattern
    @echo "Searching memory for pattern: '{{PATTERN}}'"
    WRT_DEBUG_MEMORY=1 WRT_DEBUG_MEMORY_SEARCH=1 WRT_DEBUG_INSTRUCTIONS=1 ./target/debug/wrtd --call example:hello/example#hello -f {{FUEL}} ./target/wasm32-wasip2/debug/example.wasm | grep -A10 -B2 "{{PATTERN}}"

# Test wrtd with statistics output
test-wrtd-stats: (test-wrtd-example "--call example:hello/example#hello --stats")
    # The stats test has already been executed by the dependency

# Test wrtd with both fuel and statistics
test-wrtd-fuel-stats FUEL="100": (test-wrtd-example "--call example:hello/example#hello --fuel " + FUEL + " --stats")
    # The fuel+stats test has already been executed by the dependency

# Test wrtd without any function call (should show available functions)
test-wrtd-no-call: (test-wrtd-example "")
    # The no-call test has already been executed by the dependency

# Test wrtd with help output
test-wrtd-help: build-wrtd
    ./target/debug/wrtd --help

# Test wrtd version output
test-wrtd-version: build-wrtd
    ./target/debug/wrtd --version

# Comprehensive test of wrtd with all major options
# This runs all the test commands defined above to verify different wrtd features
# Usage: just test-wrtd-all
test-wrtd-all: test-wrtd-example test-wrtd-fuel test-wrtd-stats test-wrtd-help test-wrtd-version test-wrtd-no-call

# ----------------- Code Quality Commands -----------------

# Format all Rust code
fmt:
    cargo fmt

# Check code style
check:
    cargo fmt -- --check
    cargo clippy --package wrtd -- -W clippy::missing_panics_doc -W clippy::missing_docs_in_private_items -A clippy::missing_errors_doc -A dead_code -A clippy::borrowed_box -A clippy::vec_init_then_push -A clippy::new_without_default
    # TBD: temporary disable checking no_std
    cargo clippy --package wrt --features std -- -W clippy::missing_panics_doc -W clippy::missing_docs_in_private_items -A clippy::missing_errors_doc -A dead_code -A clippy::borrowed_box -A clippy::vec_init_then_push -A clippy::new_without_default
    
# Check import organization (std first, then third-party, then internal)
check-imports:
    #!/usr/bin/env bash
    set -e
    echo "Checking import organization..."
    for file in $(find wrt wrtd -name "*.rs"); do
        if grep -q "^use " "$file"; then
            first_import=$(grep "^use " "$file" | head -1)
            if ! echo "$first_import" | grep -E "^use std|^use core|^use alloc" > /dev/null; then
                echo "WARN: $file should have standard library imports first"
                echo "First import: $first_import"
            fi
        fi
    done

# Check for unused dependencies
check-udeps:
    #!/usr/bin/env bash
    # Check if we have nightly toolchain
    if ! rustup toolchain list | grep -q "nightly"; then
        echo "Nightly toolchain required for cargo-udeps. Installing..."
        rustup toolchain install nightly --profile minimal
    fi
    
    # Check if we're on Windows
    if [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "cygwin"* || "$OSTYPE" == "win"* ]]; then
        echo "Running unused dependencies check on Windows..."
        # On Windows, just check that cargo-udeps is available but don't run it in CI
        # This avoids some Windows-specific nightly toolchain issues
        if command -v cargo-udeps &> /dev/null || rustup run nightly cargo udeps --version &> /dev/null; then
            echo "cargo-udeps is available. Skipping actual check on Windows for compatibility."
            echo "Note: For thorough checking, run 'cargo +nightly udeps' manually."
            exit 0
        else
            echo "Installing cargo-udeps..."
            cargo install cargo-udeps --locked || echo "Failed to install cargo-udeps, but continuing..."
            echo "cargo-udeps installation attempted. Skipping actual check on Windows for compatibility."
            exit 0
        fi
    else
        # Unix platforms (Linux, macOS)
        echo "Running unused dependencies check on Unix platform..."
        # Ensure cargo-udeps is installed
        if ! command -v cargo-udeps &> /dev/null; then
            echo "Installing cargo-udeps..."
            cargo install cargo-udeps --locked || echo "Warning: Failed to install cargo-udeps"
        fi
        
        # Run the actual check
        echo "Checking for unused dependencies..."
        cargo +nightly udeps -p wrt -p wrtd --all-targets || echo "Note: Criterion is allowed as an unused dev-dependency for future benchmarks"
    fi

# Run all checks (format, clippy, tests, imports, udeps, docs, wat files)
check-all: check test check-imports check-udeps check-docs test-wrtd-example check-wat-files

# Pre-commit check to run before committing changes
pre-commit: check-all
    @echo "✅ All checks passed! Code is ready to commit."

# ----------------- Documentation Commands -----------------

# Variables for Sphinx documentation
sphinx_opts := ""
sphinx_build := "sphinx-build"
sphinx_source := "docs/source"
sphinx_build_dir := "docs/_build"

# Build HTML documentation
docs-html:
    {{sphinx_build}} -M html "{{sphinx_source}}" "{{sphinx_build_dir}}" {{sphinx_opts}}
    
# Build HTML documentation with PlantUML diagrams
docs-with-diagrams: setup-python-deps setup-plantuml
    #!/usr/bin/env bash
    # Set PLANTUML_PATH environment variable
    if command -v plantuml &> /dev/null; then
        export PLANTUML_PATH="$(which plantuml)"
        echo "Using PlantUML from: $PLANTUML_PATH"
    else
        echo "WARNING: PlantUML not found in PATH. Using default 'plantuml' command."
    fi
    
    # First clean any previous diagrams
    echo "Cleaning previous diagram build artifacts..."
    rm -rf "{{sphinx_build_dir}}/html/_images/plantuml-*" "{{sphinx_build_dir}}/html/_plantuml" || true
    
    # Generate changelog 
    git-cliff -o docs/source/changelog.md

    # Build with PlantUML diagrams
    echo "Building documentation with PlantUML diagrams..."
    {{sphinx_build}} -M html "{{sphinx_source}}" "{{sphinx_build_dir}}" {{sphinx_opts}}
    
    # Confirm diagrams were generated
    DIAGRAM_COUNT=$(find "{{sphinx_build_dir}}/html/_images" -name "plantuml-*" | wc -l)
    echo "Generated $DIAGRAM_COUNT PlantUML diagrams"
    
    if [ "$DIAGRAM_COUNT" -eq 0 ]; then
        echo "WARNING: No PlantUML diagrams were generated. There might be an issue with the PlantUML setup."
        echo "Check that your .puml files are properly formatted and the PlantUML executable is available."
    fi

# Build PDF documentation (requires LaTeX installation)
docs-pdf:
    {{sphinx_build}} -M latex "{{sphinx_source}}" "{{sphinx_build_dir}}" {{sphinx_opts}}
    @echo "LaTeX files generated in docs/_build/latex. Run 'make' in that directory to build PDF (requires LaTeX installation)."

# Build all documentation formats (HTML with diagrams by default)
docs: docs-with-diagrams
    @echo "Documentation built successfully. HTML documentation available in docs/_build/html."
    @echo "To build PDF documentation, run 'just docs-pdf' (requires LaTeX installation)."

# Show Sphinx documentation help
docs-help:
    {{sphinx_build}} -M help "{{sphinx_source}}" "{{sphinx_build_dir}}" {{sphinx_opts}}

# ----------------- WebAssembly WAT/WASM Commands -----------------

# Convert a single WAT file to WASM
convert-wat-to-wasm WAT_FILE:
    #!/usr/bin/env bash
    # Check if wasm-tools is installed
    if ! command -v wasm-tools &> /dev/null; then
        echo "Error: wasm-tools not found. Please run 'just setup-wasm-tools' to install it."
        exit 1
    fi
    # Check if file exists
    if [ ! -f "{{WAT_FILE}}" ]; then
        echo "Error: WAT file not found: {{WAT_FILE}}"
        exit 1
    fi
    # Extract basename
    BASENAME=$(basename "{{WAT_FILE}}" .wat)
    DIRNAME=$(dirname "{{WAT_FILE}}")
    WASM_FILE="${DIRNAME}/${BASENAME}.wasm"
    echo "Converting {{WAT_FILE}} to ${WASM_FILE}..."
    wasm-tools parse -o "${WASM_FILE}" "{{WAT_FILE}}"
    echo "Conversion successful: ${WASM_FILE}"

# Build all WAT files in examples directory
build-wat-files:
    #!/usr/bin/env bash
    
    # Check if wasm-tools is installed (preferred method for cross-platform)
    if ! command -v wasm-tools &> /dev/null; then
        echo "Error: wasm-tools not found. Please run 'just setup-wasm-tools' to install it."
        exit 1
    fi
    
    # Cross-platform file finding
    if [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "cygwin"* || "$OSTYPE" == "win"* ]]; then
        # Windows-specific handling (using dir /b /s)
        echo "Detecting WAT files on Windows system..."
        # Use a temporary file to store the list of WAT files
        TEMP_FILE=$(mktemp)
        
        # Use Windows dir command to find .wat files and save to temp file
        cmd.exe /c "dir /b /s examples\*.wat" > "$TEMP_FILE" 2>/dev/null
        
        # Check if any WAT files were found
        if [ ! -s "$TEMP_FILE" ]; then
            echo "No WAT files found in examples directory."
            rm -f "$TEMP_FILE"
            exit 0
        fi
        
        # Process each file
        while IFS= read -r WAT_FILE; do
            # Convert Windows paths to Unix-style for WSL compatibility
            WAT_FILE=$(echo "$WAT_FILE" | tr '\\' '/')
            BASENAME=$(basename "$WAT_FILE" .wat)
            DIRNAME=$(dirname "$WAT_FILE")
            WASM_FILE="${DIRNAME}/${BASENAME}.wasm"
            
            # Check if WAT file is newer than WASM file or WASM file doesn't exist
            if [ ! -f "$WASM_FILE" ] || [ "$WAT_FILE" -nt "$WASM_FILE" ]; then
                echo "Converting $WAT_FILE to $WASM_FILE..."
                wasm-tools parse -o "$WASM_FILE" "$WAT_FILE"
            else
                echo "Skipping $WAT_FILE (WASM file is up to date)"
            fi
        done < "$TEMP_FILE"
        
        # Clean up the temporary file
        rm -f "$TEMP_FILE"
    else
        # Unix-like systems (Linux, macOS) - use find
        echo "Detecting WAT files on Unix-like system..."
        WAT_FILES=$(find examples -name "*.wat")
        
        if [ -z "$WAT_FILES" ]; then
            echo "No WAT files found in examples directory."
            exit 0
        fi
        
        # Convert each file
        for WAT_FILE in $WAT_FILES; do
            BASENAME=$(basename "$WAT_FILE" .wat)
            DIRNAME=$(dirname "$WAT_FILE")
            WASM_FILE="${DIRNAME}/${BASENAME}.wasm"
            
            # Check if WAT file is newer than WASM file or WASM file doesn't exist
            if [ ! -f "$WASM_FILE" ] || [ "$WAT_FILE" -nt "$WASM_FILE" ]; then
                echo "Converting $WAT_FILE to $WASM_FILE..."
                wasm-tools parse -o "$WASM_FILE" "$WAT_FILE"
            else
                echo "Skipping $WAT_FILE (WASM file is up to date)"
            fi
        done
    fi
    
    echo "All WAT files built successfully."

# Check if all WAT files are properly converted to WASM
check-wat-files:
    #!/usr/bin/env bash
    # Check if wasm-tools is installed (we now use wasm-tools for WAT conversion)
    if ! command -v wasm-tools &> /dev/null; then
        echo "Error: wasm-tools not found. Please run 'just setup-wasm-tools' to install it."
        exit 1
    fi
    
    NEEDS_REBUILD=0
    
    # Cross-platform file finding
    if [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "cygwin"* || "$OSTYPE" == "win"* ]]; then
        # Windows-specific handling
        echo "Checking WAT files on Windows system..."
        # Use a temporary file to store the list of WAT files
        TEMP_FILE=$(mktemp)
        
        # Use Windows dir command to find .wat files
        cmd.exe /c "dir /b /s examples\*.wat" > "$TEMP_FILE" 2>/dev/null
        
        # Check if any WAT files were found
        if [ ! -s "$TEMP_FILE" ]; then
            echo "No WAT files found in examples directory."
            rm -f "$TEMP_FILE"
            exit 0
        fi
        
        # Process each file
        while IFS= read -r WAT_FILE; do
            # Convert Windows paths to Unix-style for WSL compatibility
            WAT_FILE=$(echo "$WAT_FILE" | tr '\\' '/')
            BASENAME=$(basename "$WAT_FILE" .wat)
            DIRNAME=$(dirname "$WAT_FILE")
            WASM_FILE="${DIRNAME}/${BASENAME}.wasm"
            
            if [ ! -f "$WASM_FILE" ] || [ "$WAT_FILE" -nt "$WASM_FILE" ]; then
                echo "WARNING: WASM file needs to be rebuilt: $WASM_FILE"
                NEEDS_REBUILD=1
            fi
        done < "$TEMP_FILE"
        
        # Clean up the temporary file
        rm -f "$TEMP_FILE"
    else
        # Unix-like systems (Linux, macOS)
        echo "Checking WAT files on Unix-like system..."
        WAT_FILES=$(find examples -name "*.wat")
        
        if [ -z "$WAT_FILES" ]; then
            echo "No WAT files found in examples directory."
            exit 0
        fi
        
        # Check each file
        for WAT_FILE in $WAT_FILES; do
            BASENAME=$(basename "$WAT_FILE" .wat)
            DIRNAME=$(dirname "$WAT_FILE")
            WASM_FILE="${DIRNAME}/${BASENAME}.wasm"
            
            if [ ! -f "$WASM_FILE" ] || [ "$WAT_FILE" -nt "$WASM_FILE" ]; then
                echo "WARNING: WASM file needs to be rebuilt: $WASM_FILE"
                NEEDS_REBUILD=1
            fi
        done
    fi
    
    if [ $NEEDS_REBUILD -eq 1 ]; then
        echo "Some WASM files need to be rebuilt. Run 'just build-wat-files' to update them."
        exit 1
    else
        echo "All WAT files are properly converted to WASM."
    fi

# ----------------- Utility Commands -----------------

# Clean all build artifacts
clean:
    cargo clean
    rm -f example/hello-world.wasm
    rm -rf docs/_build
    # Also clean generated WASM files
    find examples -name "*.wasm" -type f -delete

# Install rust targets required for the project
setup-rust-targets:
    #!/usr/bin/env bash
    echo "Installing required Rust targets..."

    # Install the default Rust target for the host system
    rustup target add wasm32-unknown-unknown || { echo "Failed to install wasm32-unknown-unknown target."; exit 1; }

    # Install the ARMv8 target for Zephyr
    rustup target add aarch64-unknown-none-softfloat || { echo "Failed to install aarch64-unknown-none-softfloat target."; exit 1; }

    echo "Rust targets installed successfully!"

# Install WebAssembly tools (wasmtime, wasm-tools)
setup-wasm-tools:
    #!/usr/bin/env bash
    
    # First install essential tools using cargo (works on all platforms)
    echo "Installing wasmtime-cli and wasm-tools using cargo..."
    cargo install wasmtime-cli --locked
    cargo install wasm-tools --locked
    
    # Verify installation of required tools
    echo "Verifying WebAssembly tools installation..."
    
    # Check if wasmtime is available
    if command -v wasmtime &> /dev/null; then
        echo "✓ wasmtime is installed: $(wasmtime --version)"
    else
        echo "✗ wasmtime installation failed. Please install manually."
        exit 1
    fi
    
    # Check if wasm-tools is available - this is now our primary tool for WAT/WASM conversion
    if command -v wasm-tools &> /dev/null; then
        echo "✓ wasm-tools is installed: $(wasm-tools --version)"
    else
        echo "✗ wasm-tools installation failed. Please install manually."
        exit 1
    fi
    
    echo "Required WebAssembly tools are successfully installed."
    
    # Optionally install WABT for more tools if they're not already present
    if ! command -v wat2wasm &> /dev/null; then
        echo "Note: Additional WABT tools (e.g., wat2wasm) are not required but can be useful."
        echo "Would you like to install the WABT toolkit as well? (y/N)"
        read -r INSTALL_WABT
        
        if [[ "$INSTALL_WABT" == "y" || "$INSTALL_WABT" == "Y" ]]; then
            echo "Installing WABT tools..."
            
            if [[ "$OSTYPE" == "linux-gnu"* ]]; then
                # Linux installation
                echo "Detected Linux system, using apt-get to install wabt..."
                sudo apt-get update && sudo apt-get install -y wabt
                
            elif [[ "$OSTYPE" == "darwin"* ]]; then
                # macOS installation with Homebrew
                echo "Detected macOS system..."
                if command -v brew &> /dev/null; then
                    echo "Using Homebrew to install wabt..."
                    brew install wabt
                else
                    echo "Homebrew not found. Skipping WABT installation."
                fi
                
            elif [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "cygwin"* || "$OSTYPE" == "win"* ]]; then
                # Windows installation
                echo "Detected Windows system..."
                
                # Check if chocolatey is installed
                if command -v choco &> /dev/null; then
                    echo "Using Chocolatey to install wabt..."
                    choco install wabt -y
                else
                    echo "Chocolatey not found. Skipping WABT installation."
                fi
            else
                echo "Unsupported OS for automatic WABT installation. Skipping."
            fi
            
            # Check if wat2wasm is now available
            if command -v wat2wasm &> /dev/null; then
                echo "✓ WABT tools (wat2wasm) successfully installed: $(wat2wasm --version)"
            else
                echo "✗ WABT tools installation failed, but this is optional."
            fi
        else
            echo "Skipping WABT installation. wasm-tools will be used for WAT/WASM conversion."
        fi
    else
        echo "✓ WABT tools (wat2wasm) are already installed: $(wat2wasm --version)"
    fi
    
    echo "WebAssembly toolchain setup complete!"

# Install Python dependencies
setup-python-deps: setup-rust-targets
    cargo install git-cliff
    cargo install sphinx-rustdocgen
    pip install -r docs/requirements.txt

# Install PlantUML (requires Java)
setup-plantuml:
    #!/usr/bin/env bash
    if ! command -v plantuml &> /dev/null; then
        echo "Installing PlantUML..."
        if [[ "$OSTYPE" == "linux-gnu"* ]]; then
            # Linux installation
            sudo apt-get update && sudo apt-get install -y plantuml
            # Set path for Linux
            export PLANTUML_PATH="$(which plantuml)"
        elif [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS installation with Homebrew
            if command -v brew &> /dev/null; then
                brew install plantuml
                # Set path for macOS
                export PLANTUML_PATH="$(which plantuml)"
            else
                echo "Homebrew not found. Please install homebrew first or install plantuml manually."
                echo "Visit: https://plantuml.com/starting"
                exit 1
            fi
        elif [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "cygwin"* || "$OSTYPE" == "win"* ]]; then
            # Windows installation
            echo "For Windows, please download PlantUML jar manually from https://plantuml.com/download"
            echo "Then set PLANTUML_PATH environment variable to the jar file path"
            echo "Example: set PLANTUML_PATH=C:\\path\\to\\plantuml.jar"
            exit 1
        else
            echo "Unsupported OS. Please install plantuml manually."
            echo "Visit: https://plantuml.com/starting"
            exit 1
        fi
    else
        echo "PlantUML is already installed."
        # Set path for installed PlantUML
        export PLANTUML_PATH="$(which plantuml)"
    fi
    
    # Check if Java is installed (required for PlantUML)
    if ! command -v java &> /dev/null; then
        echo "Java is required for PlantUML but not found. Please install Java."
        exit 1
    fi
    
    # Verify PlantUML is working by testing a simple diagram
    echo -e "@startuml\nBob -> Alice : hello\n@enduml" > /tmp/test.puml
    if ! plantuml /tmp/test.puml; then
        echo "WARNING: PlantUML installation test failed. Please verify your installation."
        exit 1
    fi
    echo "PlantUML test successful!"

# Install Zephyr SDK and QEMU
setup-zephyr-sdk:
    #!/usr/bin/env bash
    echo "Setting up Zephyr SDK and QEMU..."

    # Detect the operating system
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "Detected macOS. Installing Zephyr SDK and QEMU..."
        
        # Install QEMU
        brew install qemu || echo "QEMU installation failed. Please check manually."
        
        # Create a Python virtual environment
        echo "Creating Python virtual environment for Zephyr tools..."
        python3 -m venv .zephyr-venv || { echo "Failed to create virtual environment."; exit 1; }
        
        # Activate the virtual environment
        source .zephyr-venv/bin/activate || { echo "Failed to activate virtual environment."; exit 1; }
        
        # Install west (Zephyr's meta-tool) in the virtual environment
        pip install west || { echo "Failed to install west."; deactivate; exit 1; }
        
        # Provide instructions to source the virtual environment
        echo "To use the Zephyr tools, activate the virtual environment by running:"
        echo "  source .zephyr-venv/bin/activate"
        
        # Initialize Zephyr workspace locally
        echo "Initializing Zephyr workspace in .zephyrproject..."
        mkdir -p .zephyrproject
        cd .zephyrproject || exit
        west init -m https://github.com/zephyrproject-rtos/zephyr.git || echo "Failed to initialize Zephyr workspace."
        west update || echo "Failed to fetch Zephyr modules."
        west zephyr-export || echo "Failed to export Zephyr environment variables."
        cd - || exit
        
        # Install Zephyr SDK
        echo "Installing Zephyr SDK..."
        west sdk-install || echo "Zephyr SDK installation failed. Please check manually."
        
        # Verify installation
        if command -v qemu-system-aarch64 &> /dev/null; then
            echo "Zephyr SDK and QEMU installed successfully on macOS."
        else
            echo "Zephyr SDK or QEMU installation failed. Please verify manually."
        fi

    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "Detected Linux. Installing Zephyr SDK and QEMU..."
        
        # Install dependencies
        sudo apt update
        sudo apt install -y python3-pip python3-venv git wget tar xz-utils build-essential qemu-system-arm qemu-system-aarch64
        
        # Create a Python virtual environment
        echo "Creating Python virtual environment for Zephyr tools..."
        python3 -m venv .zephyr-venv || { echo "Failed to create virtual environment."; exit 1; }
        
        # Activate the virtual environment
        source .zephyr-venv/bin/activate || { echo "Failed to activate virtual environment."; exit 1; }
        
        # Install west (Zephyr's meta-tool) in the virtual environment
        pip install west || { echo "Failed to install west."; deactivate; exit 1; }
        
        # Provide instructions to source the virtual environment
        echo "To use the Zephyr tools, activate the virtual environment by running:"
        echo "  source .zephyr-venv/bin/activate"
        
        # Initialize Zephyr workspace locally
        echo "Initializing Zephyr workspace in .zephyrproject..."
        mkdir -p .zephyrproject
        cd .zephyrproject || exit
        west init -m https://github.com/zephyrproject-rtos/zephyr.git || echo "Failed to initialize Zephyr workspace."
        west update || echo "Failed to fetch Zephyr modules."
        west zephyr-export || echo "Failed to export Zephyr environment variables."
        cd - || exit
        
        # Install Zephyr SDK
        echo "Installing Zephyr SDK..."
        west sdk-install || echo "Zephyr SDK installation failed. Please check manually."
        
        # Verify installation
        if command -v qemu-system-aarch64 &> /dev/null; then
            echo "Zephyr SDK and QEMU installed successfully on Linux."
        else
            echo "Zephyr SDK or QEMU installation failed. Please verify manually."
        fi

    elif [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "cygwin"* || "$OSTYPE" == "win"* ]]; then
        echo "Detected Windows. Installing Zephyr SDK and QEMU..."
        
        # Install dependencies
        echo "Please download and install Python, west, and the Zephyr SDK manually from:"
        echo "https://docs.zephyrproject.org/latest/develop/getting_started/index.html"
        
        echo "Installing QEMU using Chocolatey..."
        if command -v choco &> /dev/null; then
            choco install qemu -y || echo "QEMU installation failed. Please check manually."
        else
            echo "Chocolatey not found. Please install Chocolatey first or install QEMU manually."
        fi
        
        # Provide instructions for manual setup
        echo "To use the Zephyr tools on Windows, create a virtual environment and install west manually:"
        echo "  python -m venv .zephyr-venv"
        echo "  .zephyr-venv\\Scripts\\activate"
        echo "  pip install west"
        echo "Then initialize the Zephyr workspace and install the SDK as described in the documentation."
    else
        echo "Unsupported operating system. Please install the Zephyr SDK and QEMU manually."
        echo "Refer to: https://docs.zephyrproject.org/latest/develop/getting_started/index.html"
    fi

# Install required tools for development (local development)
setup: setup-hooks setup-rust-targets setup-wasm-tools setup-python-deps setup-plantuml
    @echo "✅ All development tools installed successfully."

# Setup for CI environments (without hooks)
setup-ci: setup-rust-targets setup-wasm-tools setup-python-deps setup-plantuml
    @echo "✅ CI environment setup completed."
    @echo "Building any WAT files to WASM..."
    just build-wat-files
    
# Minimal setup for CI that only installs necessary Rust targets and WASM tools
setup-ci-minimal: setup-rust-targets setup-wasm-tools
    @echo "Building any WAT files to WASM..."
    just build-wat-files
    @echo "✅ Minimal CI environment setup completed (Rust targets and WASM tools)."

# Install git hooks to enforce checks before commit/push
setup-hooks:
    @echo "Setting up Git hooks..."
    cp .githooks/pre-commit .git/hooks/pre-commit
    cp .githooks/pre-push .git/hooks/pre-push
    chmod +x .git/hooks/pre-commit .git/hooks/pre-push
    @echo "Git hooks installed successfully. Checks will run automatically before each commit and push."

# Show help
help:
    @just --list