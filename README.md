# FDT Parser

[![Crates.io](https://img.shields.io/crates/v/fdt-parser.svg)](https://crates.io/crates/fdt-parser)
[![Documentation](https://docs.rs/fdt-parser/badge.svg)](https://docs.rs/fdt-parser)
[![License](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](https://github.com/drivercraft/fdt-parser/blob/main/LICENSE)

A pure-Rust, `#![no_std]` Flattened Device Tree (FDT) parser library and CLI tool based on the devicetree-specification-v0.4.

## Overview

This project provides a comprehensive device tree blob (DTB) parser with both direct and cached parsing capabilities, specialized node types, and extensive test coverage. It is designed for embedded systems and kernel development where device tree parsing is required.

## Features

- **`no_std` Compatible**: Works in embedded environments without standard library
- **Dual Parsing Modes**:
  - Direct walk parser for memory-efficient parsing
  - Cached indexing parser for fast repeated lookups
- **Comprehensive PCI Support**: Full PCI bridge implementation with interrupt mapping
- **Complete Device Tree Support**: Memory reservations, aliases, properties, and hierarchies
- **Robust Error Handling**: Detailed error types with context information
- **Extensive Testing**: Comprehensive test coverage across all components
- **Memory Efficient**: Optimized for embedded systems with limited resources

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
fdt-parser = "0.5.0"
```

### CLI Tool Usage

```bash
# Convert a DTB file to DTS format
cargo run -p dtb-tool -- --input <dtb_file> --output <dts_file>

# Example:
cargo run -p dtb-tool -- --input bcm2711-rpi-4-b.dtb --output rpi4.dts
```

### Basic Usage

```rust
use fdt_parser::Fdt;

// Parse a device tree from bytes
let fdt = Fdt::from_bytes(&dtb_data)?;

// Access the root node
let root = fdt.root();

// Find nodes by path
let chosen = fdt.find_node("/chosen")?;

// Find nodes by compatible string
let pci_nodes = fdt.find_compatible(&["pci-host-ecam-generic"]);

// Access properties
let model = root.find_property("model")?.str()?;
let reg = root.reg()?;

// Access raw slice data
let raw_data = fdt.as_slice();
```

### PCI Node Usage

```rust
use fdt_parser::cache::node::{Node, PciSpace};

// Find PCI host bridge
let pci_nodes = fdt.find_compatible(&["pci-host-ecam-generic"]);
let pci_node = pci_nodes.first().unwrap();

if let Node::Pci(pci) = pci_node {
    // Get PCI address ranges
    for range in pci.ranges().unwrap() {
        match range.space {
            PciSpace::IO => println!("IO space: 0x{:x}-0x{:x}",
                                   range.cpu_address,
                                   range.cpu_address + range.size),
            PciSpace::Memory32 => println!("32-bit memory: 0x{:x}-0x{:x}",
                                         range.cpu_address,
                                         range.cpu_address + range.size),
            PciSpace::Memory64 => println!("64-bit memory: 0x{:x}-0x{:x}",
                                         range.cpu_address,
                                         range.cpu_address + range.size),
        }
    }

    // Resolve PCI device interrupts
    let interrupt_info = pci.child_interrupts(0, 2, 0, 1)?; // bus=0, device=2, func=0, pin=INTB
    for irq in &interrupt_info.irqs {
        println!("PCI interrupt: {}", irq);
    }
}
```

### Memory Reservation Blocks

```rust
// List all memory reservation blocks
for rsv in fdt.memory_reservation_blocks() {
    println!("Reserved: 0x{:x}-0x{:x}", rsv.address, rsv.address + rsv.size);
}
```

## Architecture

The project consists of three main crates:

### fdt-parser (Core Library)

The core parsing library with two main implementations:

1. **Base Parser** (`base/`): Direct parsing that walks the FDT structure with minimal memory overhead
2. **Cached Parser** (`cache/`): Index/cached representation for faster repeated lookups

#### Key Components

- **Header Parsing**: FDT header structure validation and metadata extraction
- **Node Management**: Hierarchical tree traversal and relationship handling
- **Property Handling**: Complete device tree property parsing with type conversion
- **Specialized Nodes**: Enhanced support for specific device types

#### Supported Node Types

- **PCI Nodes**: Complete PCI bridge support with:
  - Address space mapping (IO, Memory32, Memory64)
  - Interrupt mapping (`interrupt-map`, `interrupt-map-mask`)
  - Range translation and prefetchable memory
  - Child interrupt resolution with fallback mechanisms
  - Bus range configuration

- **Memory Nodes**: Memory reservation block parsing and iteration
- **Interrupt Controllers**: Complete interrupt hierarchy support with parent/child relationships
- **Clock Nodes**: Clock provider and consumer parsing with cell configuration
- **Chosen Nodes**: Boot parameter and chosen configuration access

### dtb-tool (CLI Tool)

Command-line tool for converting device tree blobs to device tree source format using clap for argument parsing:

```bash
# Convert a DTB file to DTS format
cargo run -p dtb-tool -- --input <dtb_file> --output <dts_file>

# Example:
cargo run -p dtb-tool -- --input bcm2711-rpi-4-b.dtb --output rpi4.dts
```

The tool outputs a human-readable device tree source (.dts) file including:
- Device tree version information (`/dts-v{version}/;`)
- Memory reservation blocks (`/memreserve/` entries)
- Node hierarchy with proper indentation based on node level
- Compatible strings for each node (filtered for non-empty values)
- Raw FDT data access capabilities

### dtb-file (Test Data)

Test device tree files and utilities for development and testing, including:
- Real device tree files from various platforms
- Custom test scenarios
- Memory reservation examples

## API Reference

### Core Types

- `Fdt`: Main device tree parser with multiple access methods
- `Node`: Device tree node with full API access and type safety
- `Property`: Device tree property with various type accessors
- `FdtError`: Comprehensive error types with detailed context

### Specialized Node Types

- `Pci`: PCI host bridge with full PCI support including ranges and interrupts
- `Memory`: Memory node with range parsing and reservation support
- `InterruptController`: Interrupt controller with hierarchy and cell configuration
- `Clock`: Clock provider/consumer with rate and parent relationships
- `Chosen`: Boot parameter node for kernel arguments and configuration

### Key Methods

#### Fdt Methods
- `find_node(path)`: Find node by absolute path
- `find_compatible(compatible_strings)`: Find nodes by compatible strings
- `memory_reservation_blocks()`: Get memory reservations as iterator
- `aliases()`: Get node aliases for path shortcuts
- `chosen()`: Get chosen boot parameters
- `all_nodes()`: Iterator over all nodes in the tree
- `root()`: Access the root node
- `as_slice()`: Get access to the raw FDT data slice
- `version()`: Get the device tree version
- `header()`: Get the FDT header information

#### Node Methods
- `name()`: Node name without path
- `find_property(name)`: Get property by name
- `reg()`: Get register information with address translation
- `compatible()`: Get compatible strings list
- `interrupts()`: Get interrupt information with parent resolution
- `parent()`: Get parent node reference
- `children()`: Get child nodes list
- `address_cells()`: Get #address-cells property
- `size_cells()`: Get #size-cells property
- `level()`: Get node depth level in the tree hierarchy

#### Property Methods
- `str()`: String property value with UTF-8 validation
- `u32()`: 32-bit integer property value (big-endian)
- `u64()`: 64-bit integer property value (big-endian)
- `data()`: Raw property data access

#### PCI-Specific Methods
- `ranges()`: Get PCI address space ranges with space type decoding
- `interrupt_map()`: Parse interrupt mapping table with mask application
- `interrupt_map_mask()`: Get interrupt mapping mask
- `child_interrupts(bus, device, function, pin)`: Resolve device interrupts
- `bus_range()`: Get PCI bus number range
- `is_pci_host_bridge()`: Check if node is a PCI host bridge

## Testing

The project includes comprehensive test coverage:

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test -p fdt-parser cache          # Cached parser tests
cargo test -p fdt-parser base           # Base parser tests
cargo test -p fdt-parser pci            # PCI-specific tests

# Run with output for debugging
cargo test -p fdt-parser -- --nocapture
```

### Test Coverage

- Comprehensive test coverage across all components
- PCI node functionality (ranges, interrupts, properties, edge cases)
- Memory reservation block parsing and iteration
- Node hierarchy and property access validation
- Error handling and boundary condition testing
- Multiple real device tree files from various platforms
- Performance and memory usage validation

## Performance Characteristics

- **Base Parser**: O(N) time complexity, minimal memory overhead
- **Cached Parser**: O(1) lookup for cached nodes, O(N) initial indexing
- **Memory Usage**: Optimized for embedded systems with allocators
- **PCI Interrupt Resolution**: O(log M) where M is number of interrupt map entries

The cached parser provides O(1) lookup for frequently accessed nodes and properties, making it suitable for performance-critical applications where the device tree is accessed repeatedly.

## Standards Compliance

This implementation follows industry standards:

- **[Devicetree Specification v0.4](https://www.devicetree.org/specifications/)**: Complete compliance with device tree format
- **[PCI Bus Binding Documentation](https://www.kernel.org/doc/Documentation/devicetree/bindings/pci/pci.txt)**: PCI bridge and interrupt mapping standards
- **[Open Firmware Interrupt Mapping Practice](https://www.devicetree.org/open-firmware/practice/imap/imap0_9d.html)**: Interrupt mapping conventions
- **[Device Tree Copyright License](https://www.devicetree.org/specifications/copyright.txt)**: Specification licensing

## Examples

The project includes several example device tree files for testing and development:

- **BCM2711 (Raspberry Pi 4)**: ARM platform with PCIe support and complex interrupt mapping
- **Phytium**: ARM platform with comprehensive PCI configuration
- **QEMU**: Virtual platform with simplified device tree for testing
- **RK3568**: ARM SoC platform with multiple device types
- **Custom**: Device trees with specific features for edge case testing

## Platform Support

- **x86_64**: Full support for development and testing
- **ARM**: Complete support for embedded platforms
- **RISC-V**: Cross-compilation support
- **AArch64**: 64-bit ARM platform support

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Add tests for new functionality
4. Ensure all tests pass (`cargo test`)
5. Format code (`cargo fmt --all`)
6. Run clippy (`cargo clippy --all`)
7. Submit a pull request

### Development Guidelines

- Follow Rust best practices and idioms
- Add comprehensive tests for new features
- Update documentation for API changes
- Ensure `no_std` compatibility
- Test with real device tree files when possible

## License

This project is licensed under the MPL-2.0 License - see the [LICENSE](LICENSE) file for details.

## Repository

https://github.com/drivercraft/fdt-parser

## Changelog

### v0.5.0 (Current)
- **PCI Support**: Complete PCI node implementation with full feature support
  - Address space types (IO, Memory32, Memory64) with prefetchable flag
  - Interrupt mapping with `interrupt-map` and `interrupt-map-mask` parsing
  - Child interrupt resolution with fallback mechanisms
  - Range translation and bus range support
- **Enhanced CLI Tool**: Updated dtb-tool with clap argument parsing and improved output formatting
- **Raw Data Access**: New `Fdt::as_slice()` method for accessing internal raw data
- **Enhanced Testing**: Comprehensive test suite with real device tree files
- **Memory Reservations**: Improved iterator support and parsing accuracy with corrected method names
- **Error Handling**: Better error context and recovery mechanisms with thiserror v2
- **Performance**: Optimized parsing algorithms and memory usage
- **Documentation**: Complete API documentation and usage examples
- **Repository Updates**: Updated repository URL to https://github.com/drivercraft/fdt-parser

### v0.4.x
- Initial cached parser implementation
- Basic node type support
- Memory reservation block parsing

### v0.3.x
- Base parser implementation
- Core device tree parsing functionality
- Property and node access methods

## Related Projects

- [Linux Device Tree](https://www.devicetree.org/): Official device tree specification and resources
- [Device Tree Compiler (DTC)](https://github.com/dgibson/dtc): Reference device tree compiler
- [Rust Embedded Working Group](https://github.com/rust-embedded/wg): Embedded Rust community

## Citation

If you use this project in research or production, please cite:

```
FDT Parser - A pure-Rust, no_std Flattened Device Tree parser
https://github.com/drivercraft/fdt-parser
```