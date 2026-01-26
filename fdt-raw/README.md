# fdt-raw

A low-level Rust library for parsing Device Tree Blob (DTB) files.

## Overview

`fdt-raw` is a pure Rust, `#![no_std]` compatible device tree parsing library based on the [Device Tree Specification v0.4](https://www.devicetree.org/specifications/). This library provides low-level access interfaces to the Flattened Device Tree (FDT) structure, suitable for embedded systems and bare-metal development environments.

## Features

- **Pure Rust Implementation**: No C language dependencies
- **`no_std` Compatible**: Suitable for bare-metal and embedded environments
- **Specification Based**: Strictly follows Device Tree Specification v0.4
- **Zero-Copy Parsing**: Operates directly on raw data, avoiding unnecessary memory allocations
- **Type Safe**: Provides strongly typed API interfaces
- **Memory Efficient**: Uses `heapless` for allocator-free collections

## Core Components

### Fdt Structure
The main FDT parser providing access to the device tree structure:
- Header information parsing
- Memory reservation block traversal
- Node tree traversal
- Property access

### Supported Node Types
- **Memory Nodes**: Parse memory region information
- **Chosen Nodes**: Access boot parameters
- **General Nodes**: Handle all other node types

### Property Parsing
- **reg Property**: Address range parsing with `#address-cells` and `#size-cells` support
- **Property Iterators**: Efficient property traversal
- **Property Value Access**: Provides various data type access methods

## Quick Start

```rust
use fdt_raw::Fdt;

// Parse FDT from byte data
let fdt = Fdt::from_bytes(&dtb_data)?;

// Iterate through root node's children
for node in fdt.root().children() {
    println!("Node name: {}", node.name()?);

    // Iterate through node properties
    for prop in node.properties() {
        println!("  Property: {}", prop.name()?);
    }
}

// Access memory reservation block
for reservation in fdt.memory_reservations() {
    println!("Reserved: 0x{:x} - 0x{:x}",
             reservation.address,
             reservation.address + reservation.size);
}
```

## Dependencies

- `heapless = "0.9"` - Allocator-free collections
- `log = "0.4"` - Logging
- `thiserror = {version = "2", default-features = false}` - Error handling

## Dev Dependencies

- `dtb-file` - Test data
- `env_logger = "0.11"` - Logging implementation

## License

This project is open source. Please see the LICENSE file in the project root directory for specific license details.

## Contributing

Issues and Pull Requests are welcome. Please ensure:

1. Code follows project formatting standards (`cargo fmt`)
2. All tests pass (`cargo test`)
3. Clippy checks pass (`cargo clippy`)

## Related Projects

- [fdt-parser](../fdt-parser/) - Higher-level cached FDT parser
- [fdt-edit](../fdt-edit/) - FDT editing and manipulation library
- [dtb-tool](../dtb-tool/) - DTB file inspection tool
- [dtb-file](../dtb-file/) - Test data package
