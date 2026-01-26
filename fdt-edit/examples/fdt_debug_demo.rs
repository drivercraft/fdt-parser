//! FDT deep debug demonstration
//!
//! Demonstrates how to use the new deep debug functionality to traverse
//! and print all nodes in the device tree.

use dtb_file::fdt_rpi_4b;
use fdt_edit::Fdt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Create FDT from RPI 4B DTB data
    let raw_data = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw_data)?;

    println!("=== FDT Basic Debug Information ===");
    // Basic debug format (compact)
    println!("{:?}", fdt);
    println!();

    println!("=== FDT Deep Debug Information ===");
    // Deep debug format (traverses all nodes)
    println!("{:#?}", fdt);

    Ok(())
}
