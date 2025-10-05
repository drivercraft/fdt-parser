# FDT Parser

[![Build & Check CI](https://github.com/drivercraft/fdt-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/drivercraft/fdt-parser/actions/workflows/ci.yml)
[![Latest version](https://img.shields.io/crates/v/fdt-parser.svg)](https://crates.io/crates/fdt-parser)
[![Documentation](https://docs.rs/fdt-parser/badge.svg)](https://docs.rs/fdt-parser)
![License](https://img.shields.io/crates/l/fdt-parser.svg)

A pure Rust, `#![no_std]` Flattened Device Tree (FDT) parser library based on [devicetree-specification-v0.4](https://github.com/devicetree-org/devicetree-specification/releases/download/v0.4/devicetree-specification-v0.4.pdf).

## Features

- **[√] Parse device tree blob** - Complete FDT structure parsing
- **[√] Dual parsing implementations** - Direct (`base`) and cached (`cache`) parsing modes
- **[√] Memory reservation handling** - Parse memory reservation blocks
- **[√] Register address translation** - Fix `reg` addresses by `range` properties
- **[√] Interrupt handling** - Find interrupt parents and parse interrupt specifications
- **[√] Clock binding support** - Parse clock providers and consumers
- **[√] Alias resolution** - Handle path aliases
- **[√] PCI bus support** - Specialized PCI host bridge parsing
- **[√] Property access** - Full access to all node properties with type-safe helpers
- **[√] Node traversal** - Hierarchical node navigation and search
- **[√] Compatible matching** - Find nodes by compatible strings
- **[√] No-std compatible** - Works in embedded environments with `alloc`

## Architecture

The library provides two parsing approaches:

### Base Parser (`base` module)

Direct parsing approach that walks the FDT structure on-demand. Uses lifetimes to avoid data copying and is memory efficient for single-pass operations.

### Cached Parser (`cache` module)

Builds an indexed representation for faster repeated lookups. Copies data into owned structures but provides O(1) access for many operations after initial parsing.

## Usage

### Basic Usage (Cached Parser)

```rust
use fdt_parser::Fdt;

let bytes = include_bytes!("path/to/device-tree.dtb");

let fdt = Fdt::from_bytes(bytes).unwrap();
println!("FDT version: {}", fdt.version());

// Access memory reservation blocks
for region in fdt.memory_reservation_blocks() {
    println!("Reserved region: {:?}", region);
}

// Traverse all nodes
for node in fdt.all_nodes() {
    let indent = "  ".repeat(node.level());
    println!("{}{} ({})", indent, node.name(), node.full_path());

    // Get compatible strings
    if !node.compatibles().is_empty() {
        println!("{}  Compatible: {:?}", indent, node.compatibles());
    }

    // Get register information
    if let Ok(reg) = node.reg() {
        println!("{}  Register: {:?}", indent, reg);
    }
}
```

### Advanced Usage

```rust
use fdt_parser::Fdt;

let fdt = Fdt::from_bytes(bytes).unwrap();

// Find nodes by path
let memory_nodes = fdt.find_nodes("/memory@");
for node in memory_nodes {
    if let fdt_parser::Node::Memory(mem) = node {
        for region in mem.regions().unwrap() {
            println!("Memory region: {:x}-{:x}", region.address, region.address + region.size);
        }
    }
}

// Find nodes by compatible strings
let uart_devices = fdt.find_compatible(&["generic-uart"]);
for uart in uart_devices {
    println!("UART device at: {}", uart.full_path());

    // Get interrupts
    if let Ok(interrupts) = uart.interrupts() {
        println!("  Interrupts: {:?}", interrupts);
    }

    // Get clocks
    if let Ok(clocks) = uart.clocks() {
        for clock in clocks {
            println!("  Clock: {} from provider {}",
                clock.name.as_deref().unwrap_or("unnamed"),
                clock.provider_name()
            );
        }
    }
}

// Access chosen node properties
if let Some(chosen) = fdt.get_node_by_path("/chosen") {
    if let fdt_parser::Node::Chosen(chosen) = chosen {
        if let Some(bootargs) = chosen.bootargs() {
            println!("Boot args: {}", bootargs);
        }
    }
}

// Use aliases to find nodes
let serial = fdt.find_aliase("serial0")
    .and_then(|path| fdt.get_node_by_path(&path));

if let Some(serial_node) = serial {
    println!("Serial console at: {}", serial_node.full_path());
}
```

### Property Access

```rust
use fdt_parser::Fdt;

let fdt = Fdt::from_bytes(bytes).unwrap();
let node = fdt.get_node_by_path("/cpus/cpu@0").unwrap();

// Access specific properties
if let Some(prop) = node.find_property("clock-frequency") {
    if let Ok(freq) = prop.u32() {
        println!("CPU frequency: {} Hz", freq);
    }
}

// String list properties
if let Some(prop) = node.find_property("compatible") {
    for compatible in prop.str_list() {
        println!("Compatible: {}", compatible);
    }
}

// Raw data access
if let Some(prop) = node.find_property("reg") {
    let raw_data = prop.raw_value();
    println!("Raw register data: {:x?}", raw_data);
}
```

## API Documentation

For comprehensive API documentation, see [docs.rs/fdt-parser](https://docs.rs/fdt-parser).

## License

Licensed under the MPL-2.0 license. See [LICENSE](../LICENSE) for details.
