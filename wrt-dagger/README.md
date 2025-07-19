# wrt-dagger - Containerized Build System

A containerized build wrapper for WRT using [Dagger](https://dagger.io/), providing reproducible builds across different environments.

## Overview

wrt-dagger is an optional module that enables running WRT builds in isolated containers. It serves as a thin wrapper around `cargo-wrt`, ensuring consistent build environments for CI/CD and development.

## Features

- **🐋 Containerized Builds**: Run builds in isolated, reproducible containers
- **🔄 Cross-Platform Consistency**: Same environment on Linux, macOS, and Windows
- **⚡ cargo-wrt Integration**: Leverages the unified WRT build system
- **🛡️ Safety Verification**: Specialized containers for ASIL compliance testing
- **🚀 CI/CD Ready**: Optimized for GitHub Actions and other CI systems

## Installation

### Prerequisites

1. **Install Dagger** (optional, falls back to local execution):
   ```bash
   # Linux/macOS
   curl -fsSL https://dl.dagger.io/dagger/install.sh | sh
   
   # Or via package manager
   brew install dagger/tap/dagger  # macOS
   ```

2. **Install wrt-dagger**:
   ```bash
   cargo install --path wrt-dagger --features dagger
   ```

## Usage

### Command Line Interface

```bash
# Basic build in container
wrt-dagger build

# Run tests in container
wrt-dagger test

# Full CI pipeline
wrt-dagger ci

# Safety verification (ASIL-D)
wrt-dagger verify --asil d

# Coverage analysis
wrt-dagger coverage

# Custom cargo-wrt command
wrt-dagger run -- verify --asil c --detailed

# Show configuration
wrt-dagger config
```

### Pre-configured Pipelines

```bash
# Optimized for CI/CD
wrt-dagger ci-pipeline

# Development environment
wrt-dagger dev-pipeline

# Safety-critical verification
wrt-dagger safety-pipeline
```

### Rust Library Usage

```rust
use wrt_dagger::{DaggerPipeline, ContainerConfigBuilder, utils};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Custom configuration
    let config = ContainerConfigBuilder::new()
        .base_image("rust:1.86-slim")
        .add_package("git")
        .env("CI", "true")
        .timeout(1800)
        .build();

    let pipeline = DaggerPipeline::new(config).await?;
    
    // Run build
    let output = pipeline.build().await?;
    println!("Build output: {}", output);
    
    // Run tests
    let test_output = pipeline.test().await?;
    println!("Test output: {}", test_output);
    
    // Safety verification
    let verify_output = pipeline.verify("d").await?;
    println!("Verification output: {}", verify_output);
    
    Ok(())
}
```

### Pre-configured Pipelines

```rust
use wrt_dagger::utils;

// CI/CD optimized pipeline
let ci_pipeline = utils::ci_pipeline().await?;
ci_pipeline.ci().await?;

// Development pipeline with caching
let dev_pipeline = utils::dev_pipeline().await?;
dev_pipeline.build().await?;

// Safety verification pipeline
let safety_pipeline = utils::safety_pipeline().await?;
safety_pipeline.verify("d").await?;
```

## Configuration

### Container Configuration

The `ContainerConfig` struct allows full customization:

```rust
use wrt_dagger::{ContainerConfig, ContainerConfigBuilder};

let config = ContainerConfigBuilder::new()
    .base_image("ubuntu:22.04")           // Base container image
    .rust_version("1.86.0")              // Rust toolchain version
    .add_package("build-essential")      // System packages
    .add_package("libssl-dev")
    .env("RUST_LOG", "debug")            // Environment variables
    .env("CARGO_INCREMENTAL", "0")
    .work_dir("/workspace")              // Working directory
    .cache_dependencies(true)            // Enable dependency caching
    .timeout(3600)                       // Timeout in seconds
    .build();
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Logging level | `info` |
| `CARGO_INCREMENTAL` | Incremental compilation | `0` |
| `CI` | CI environment flag | - |
| `CARGO_TERM_COLOR` | Terminal colors | `always` |

## CI/CD Integration

### GitHub Actions

```yaml
name: Containerized Build
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Dagger
        run: |
          curl -fsSL https://dl.dagger.io/dagger/install.sh | sh
          echo "$HOME/.local/bin" >> $GITHUB_PATH
      
      - name: Install wrt-dagger
        run: cargo install --path wrt-dagger --features dagger
      
      - name: Run CI Pipeline
        run: wrt-dagger ci-pipeline
      
      - name: Safety Verification
        run: wrt-dagger verify --asil d
```

### Local Development

```bash
# Quick development build
wrt-dagger dev-pipeline

# Custom environment
wrt-dagger --base-image rust:1.86-alpine build

# Debug with verbose output
wrt-dagger --verbose test
```

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   wrt-dagger    │───▶│  Dagger Engine  │───▶│   Container     │
│   (CLI/Lib)     │    │   (Optional)    │    │   + cargo-wrt   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       │                       ▼
         │                       │              ┌─────────────────┐
         │                       │              │ WRT Build       │
         │                       │              │ (wrt-build-core)│
         │                       │              └─────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐    ┌─────────────────┐
│ Local Fallback  │    │ Container Cache │
│ (cargo-wrt)     │    │ (Dependencies)  │
└─────────────────┘    └─────────────────┘
```

## Features

### Optional Dagger Integration

The module can operate in two modes:

1. **With Dagger** (`--features dagger`): Full containerization
2. **Without Dagger**: Falls back to local `cargo-wrt` execution

### Specialized Pipelines

- **CI Pipeline**: Optimized for continuous integration
- **Dev Pipeline**: Fast builds with dependency caching
- **Safety Pipeline**: ASIL-D verification with formal tools

### Container Optimization

- Dependency caching for faster builds
- Multi-stage builds for minimal image size
- Security-conscious container configuration
- Cross-platform base images

## Troubleshooting

### Common Issues

**Dagger not found:**
```bash
# Install Dagger
curl -fsSL https://dl.dagger.io/dagger/install.sh | sh
```

**Build timeout:**
```bash
# Increase timeout
wrt-dagger --timeout 7200 build
```

**Container image issues:**
```bash
# Use different base image
wrt-dagger --base-image rust:1.86-slim build
```

**Fallback mode:**
```bash
# Without Dagger (uses local cargo-wrt)
cargo install --path wrt-dagger  # No --features dagger
wrt-dagger build  # Falls back to local execution
```

### Debug Mode

```bash
# Enable verbose logging
wrt-dagger --verbose build

# Check configuration
wrt-dagger config

# Test connection
dagger version  # If using Dagger
```

## Development

### Building

```bash
# With Dagger support
cargo build --features dagger

# Without Dagger (local fallback only)
cargo build
```

### Testing

```bash
# Run tests
cargo test

# Integration tests (requires Dagger)
cargo test --features dagger -- --test-threads=1
```

## Integration with WRT Build System

wrt-dagger seamlessly integrates with the WRT unified build system:

1. **Delegates to cargo-wrt**: All build operations use `cargo-wrt` commands
2. **Preserves functionality**: Same commands, containerized execution
3. **Maintains compatibility**: Works with existing workflows
4. **Enhances reliability**: Consistent environments across platforms

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.