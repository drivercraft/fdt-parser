# fdt-edit

A high-level Rust library for creating, editing, and encoding Flattened Device Tree (FDT) structures.

## Overview

`fdt-edit` is a feature-rich device tree manipulation library built on top of `fdt-raw`. It provides comprehensive functionality for creating new device trees from scratch, modifying existing device trees, and encoding the edited device trees into standard DTB format.

## Features

- **Complete device tree editing**: Full CRUD operations for nodes and properties
- **Type-safe node operations**: Specialized node types (clocks, memory, PCI, interrupt controllers, etc.)
- **Efficient encoder**: Converts in-memory device tree structures to standard DTB format
- **phandle management**: Automatic phandle allocation and reference management
- **Memory reservation support**: Complete memory reservation region operations
- **`no_std` compatible**: Suitable for embedded environments

## Core Components

### Fdt Structure
An editable device tree container that:
- Parses from raw DTB data
- Creates new empty device trees
- Manages phandle cache
- Encodes to DTB format

### Node System
Supports multiple specialized node types:
- **Clock nodes**: Clock sources and clock consumers
- **Memory nodes**: Memory region definitions
- **PCI nodes**: PCI buses and devices
- **Interrupt controllers**: Interrupt mapping and management
- **Generic nodes**: Customizable node types

### Property System
- **Type-safe properties**: Support for various data types
- **Automatic property management**: Intelligent property CRUD operations
- **Formatted display**: Friendly node and property display

## Quick Start

```rust
use fdt_edit::Fdt;

// Parse existing DTB from bytes
let raw_data = include_bytes!("path/to/device-tree.dtb");
let fdt = Fdt::from_bytes(&raw_data)?;

// Access nodes by path
let node = fdt.get_by_path("/chosen");
if let Some(chosen) = node {
    println!("Found chosen node: {}", chosen.name());
}

// Encode back to DTB format
let dtb_data = fdt.encode();
std::fs::write("output.dtb", dtb_data.as_bytes())?;
```

### Node Traversal and Searching

```rust
use fdt_edit::{Fdt, NodeKind};

let fdt = Fdt::from_bytes(&dtb_data)?;

// Iterate through all nodes
for node in fdt.all_nodes() {
    println!("Node: {} at path {}", node.name(), node.path());

    // Match specialized node types
    match node.as_ref() {
        NodeKind::Memory(mem) => {
            println!("  Memory node with regions:");
            for region in mem.regions() {
                println!("    address=0x{:x}, size=0x{:x}", region.address, region.size);
            }
        }
        NodeKind::Clock(clock) => {
            println!("  Clock node: {} (#clock-cells={})", clock.name(), clock.clock_cells);
        }
        NodeKind::Pci(pci) => {
            if let Some(range) = pci.bus_range() {
                println!("  PCI bus range: {:?}", range);
            }
        }
        _ => {
            println!("  Generic node");
        }
    }
}

// Find nodes by path pattern
let virtio_nodes: Vec<_> = fdt.find_by_path("/virtio_mmio").collect();
println!("Found {} virtio_mmio nodes", virtio_nodes.len());
```

### Node Modification and Creation

```rust
use fdt_edit::{Fdt, Node};

let mut fdt = Fdt::from_bytes(&dtb_data)?;

// Create new node manually
let mut new_node = Node::new("test-device@12340000");
// Add properties (API in development)
// new_node.add_property("compatible", &["vendor,test-device"]);
// new_node.add_property("reg", &[0x12340000u64, 0x1000u64]);

// Add to root node
fdt.root.add_child(new_node);

// Remove existing node
if fdt.get_by_path("/psci").is_some() {
    let removed = fdt.remove_node("/psci")?;
    println!("Removed psci node: {}", removed.unwrap().name());
}

// Save the modified device tree
let modified_dtb = fdt.encode();
std::fs::write("modified.dtb", modified_dtb.as_bytes())?;
```

### Specialized Node Access

```rust
use fdt_edit::{Fdt, NodeKind};

let fdt = Fdt::from_bytes(&dtb_data)?;

// Find and work with memory nodes
for node in fdt.all_nodes() {
    if let NodeKind::Memory(mem) = node.as_ref() {
        let regions = mem.regions();
        if !regions.is_empty() {
            println!("Memory node '{}' has {} regions:", mem.name(), regions.len());
            for (i, region) in regions.iter().enumerate() {
                println!("  Region {}: 0x{:x}-0x{:x}", i, region.address, region.address + region.size);
            }
        }
    }
}

// Find clock nodes
let mut clock_count = 0;
for node in fdt.all_nodes() {
    if let NodeKind::Clock(clock) = node.as_ref() {
        clock_count += 1;
        println!("Clock {}: cells={}, output-names={:?}",
                 clock.name(),
                 clock.clock_cells,
                 clock.clock_output_names);
    }
}
```

### Display as Device Tree Source

```rust
use fdt_edit::Fdt;

let fdt = Fdt::from_bytes(&dtb_data)?;

// Display as DTS format (including memory reservations)
println!("{}", fdt);
// Output will show:
// /dts-v1/;
// /memreserve/ 0x80000000 0x100000;
// / {
//     #address-cells = <0x2>;
//     #size-cells = <0x2>;
//     compatible = "qemu,arm64";
//     ...
// };
```

## Current Status

This library is under active development. Currently supported features:
- ✅ Parse DTB files into editable structures
- ✅ Encode device trees back to DTB format
- ✅ Display device trees in DTS format
- ✅ Access to memory reservations
- 🚧 Node editing APIs (in development)

## Dependencies

- `fdt-raw` - Low-level FDT parsing library
- `log = "0.4"` - Logging support
- `enum_dispatch = "0.3.13"` - Enum dispatch optimization

## Dev Dependencies

- `dtb-file` - Test data
- `env_logger = "0.11"` - Logger implementation

## Testing

The library includes comprehensive tests that verify round-trip compatibility:

```bash
cargo test
```

The main test (`test_parse_and_rebuild`) ensures that:
1. A DTB file can be parsed successfully
2. The parsed structure can be encoded back to DTB
3. The original and rebuilt DTB files produce identical DTS output when using `dtc`

## License

This project is licensed under open source licenses. Please see the LICENSE file in the project root for specific license types.

## Contributing

Issues and Pull Requests are welcome. Please ensure:

1. Code follows the project's formatting standards (`cargo fmt`)
2. All tests pass (`cargo test`)
3. Clippy checks pass (`cargo clippy`)
4. New features include appropriate test cases

## Related Projects

- [fdt-raw](../fdt-raw/) - Low-level FDT parsing library
- [fdt-parser](../fdt-parser/) - High-level cached FDT parser
- [dtb-tool](../dtb-tool/) - DTB file inspection tool
- [dtb-file](../dtb-file/) - Test data package

## Examples

More usage examples can be found in the source code test files, particularly in `tests/edit.rs`.