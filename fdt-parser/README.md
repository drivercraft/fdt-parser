# Device Tree FDT format parser

Base on [devicetree-specification-v0.4](https://github.com/devicetree-org/devicetree-specification/releases/download/v0.4/devicetree-specification-v0.4.pdf)

## Usage

```rust
fn main() {
    let bytes = include_bytes!("../dtb/bcm2711-rpi-4-b.dtb");

    let fdt = Fdt::from_bytes(bytes).unwrap();
    println!("version: {}", fdt.version());
    for region in fdt.reserved_memory_regions() {
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
}

```
