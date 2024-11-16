# FDT Parser

[![Build & Check CI](https://github.com/qclic/fdt-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/qclic/fdt-parser/actions/workflows/ci.yml)
[![Latest version](https://img.shields.io/crates/v/fdt-parser.svg)](https://crates.io/crates/fdt-parser)
[![Documentation](https://docs.rs/fdt-parser/badge.svg)](https://docs.rs/fdt-parser)
![License](https://img.shields.io/crates/l/fdt-parser.svg)

Base on [devicetree-specification-v0.4](https://github.com/devicetree-org/devicetree-specification/releases/download/v0.4/devicetree-specification-v0.4.pdf)

## Advance features

- [√] Parse device tree blob
- [√] Fix `reg` address by `range`
- [√] Find interrupt parent
- [√] Find clocks
- [√] Handle `aliases`
- [√] PCI bus

## Usage

```rust
use fdt_parser::Fdt;

let bytes = include_bytes!("../../dtb/bcm2711-rpi-4-b.dtb");

let fdt = Fdt::from_bytes(bytes).unwrap();
println!("version: {}", fdt.version());
for region in fdt.memory_reservation_block() {
    println!("region: {:?}", region);
}

for node in fdt.all_nodes() {
    let space = " ".repeat((node.level - 1) * 4);
    println!("{}{}", space, node.name());

    if let Some(cap) = node.compatible() {
        println!("{} -compatible: ", space);
        for cap in cap {
            println!("{}     {:?}", space, cap);
        }
    }

    if let Some(reg) = node.reg() {
        println!("{} - reg: ", space);
        for cell in reg {
            println!("{}     {:?}", space, cell);
        }
    }
}

```
