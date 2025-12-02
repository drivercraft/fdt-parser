#![cfg(not(target_os = "none"))]

#[macro_use]
extern crate log;

use dtb_file::*;
use fdt_raw::*;
use std::sync::Once;

fn init_logging() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    });
}

#[test]
fn test_phandle_display() {
    let phandle = Phandle::from(42);
    assert_eq!(format!("{}", phandle), "<0x2a>");
}

#[test]
fn test_new() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    info!("ver: {:#?}", fdt.header().version);
}

#[test]
fn test_all_nodes() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    for node in fdt.all_nodes() {
        info!("node: {}", node.name());
    }
}

#[test]
fn test_node_context() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    for node in fdt.all_nodes() {
        info!(
            "node: {} (level={}, addr_cells={}, size_cells={}, parent_addr_cells={}, parent_size_cells={}, ranges_count={})",
            node.name(),
            node.level(),
            node.address_cells,
            node.size_cells,
            node.reg_address_cells(),
            node.reg_size_cells(),
            node.context.ranges.len()
        );
    }
}

#[test]
fn test_node_properties() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    for node in fdt.all_nodes() {
        info!("node: {}", node.name());
        for prop in node.properties() {
            match &prop {
                Property::AddressCells(v) => info!("  #address-cells = {}", v),
                Property::SizeCells(v) => info!("  #size-cells = {}", v),
                Property::InterruptCells(v) => info!("  #interrupt-cells = {}", v),
                Property::Status(s) => info!("  status = {:?}", s),
                Property::Phandle(p) => info!("  phandle = {}", p),
                Property::LinuxPhandle(p) => info!("  linux,phandle = {}", p),
                Property::InterruptParent(p) => info!("  interrupt-parent = {}", p),
                Property::Model(s) => info!("  model = \"{}\"", s),
                Property::DeviceType(s) => info!("  device_type = \"{}\"", s),
                Property::Compatible(iter) => {
                    let strs: Vec<_> = iter.clone().collect();
                    info!("  compatible = {:?}", strs);
                }
                Property::ClockNames(iter) => {
                    let strs: Vec<_> = iter.clone().collect();
                    info!("  clock-names = {:?}", strs);
                }
                Property::Reg(data) => info!("  reg ({} bytes)", data.len()),
                Property::Ranges(data) => info!("  ranges ({} bytes)", data.len()),
                Property::Interrupts(data) => info!("  interrupts ({} bytes)", data.len()),
                Property::Clocks(data) => info!("  clocks ({} bytes)", data.len()),
                Property::DmaCoherent => info!("  dma-coherent"),
                Property::Unknown(raw) => {
                    if let Some(s) = raw.as_str() {
                        info!("  {} = \"{}\"", raw.name(), s);
                    } else if let Some(v) = raw.as_u32() {
                        info!("  {} = {:#x}", raw.name(), v);
                    } else if raw.is_empty() {
                        info!("  {} (empty)", raw.name());
                    } else {
                        info!("  {} ({} bytes)", raw.name(), raw.len());
                    }
                }
            }
        }
    }
}
