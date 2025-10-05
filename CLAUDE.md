# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust workspace containing a Flattened Device Tree (FDT) parser library and associated tools. The project implements a pure-Rust, `#![no_std]` parser for device tree blob files based on the devicetree-specification-v0.4.

## Workspace Structure

This is a Cargo workspace with three main crates:

- `fdt-parser/`: Core library crate for parsing FDT files
- `dtb-tool/`: CLI tool for inspecting device tree blobs
- `dtb-file/`: Test data and sample DTB files

## Common Commands

### Build and Test
```bash
# Build all workspace members
cargo build

# Build specific package
cargo build -p fdt-parser
cargo build -p dtb-tool

# Run tests (only works on x86_64-unknown-linux-gnu target)
cargo test

# Build for different targets (used in CI)
cargo build -p fdt-parser --target x86_64-unknown-none
cargo build -p fdt-parser --target riscv64gc-unknown-none-elf
cargo build -p fdt-parser --target aarch64-unknown-none-softfloat
```

### Code Quality
```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --manifest-path ./fdt-parser/Cargo.toml --all-features -- -A clippy::new_without_default

# Check formatting without modifying files
cargo fmt --all -- --check
```

### Running the CLI Tool
```bash
# Run the dtb-tool CLI
cargo run -p dtb-tool -- <dtb_file>
```

## Architecture

The `fdt-parser` crate provides two main parsing implementations:

1. **Base Parser** (`base/`): Direct parsing approach that walks the FDT structure
2. **Cached Parser** (`cache/`): Builds an index/cached representation for faster repeated lookups

Key modules:
- `header.rs`: FDT header structure parsing
- `property.rs`: Device tree property handling
- `data.rs`: Data access utilities
- `base/node/`: Node-specific parsing logic (memory, interrupts, clocks, etc.)
- `cache/`: Cached FDT representation with node management

## CI Configuration

The project uses GitHub Actions with:
- Rust nightly toolchain
- Multi-target builds (Linux, no-std x86_64, RISC-V, ARM)
- Code formatting checks
- Clippy linting
- Unit tests (Linux target only)

## Development Notes

- The project is `#![no_std]` compatible and uses `alloc` for dynamic memory
- Uses `heapless` for collections without allocator
- Error handling via `thiserror` crate
- Test DTB files are included in `dtb-file/src/dtb/` for development