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
fn test_fdt_display() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();
    let output = format!("{}", fdt);
    info!("FDT Display:\n{}", output);

    // 验证输出包含预期的结构
    assert!(output.contains("/dts-v1/;"));
    assert!(output.contains("/ {"));
    assert!(output.contains("psci {"));
    assert!(output.contains("compatible ="));
}

#[test]
fn test_fdt_debug() {
    init_logging();
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();
    let output = format!("{:?}", fdt);
    info!("FDT Debug:\n{}", output);

    // 验证 Debug 输出包含预期的结构
    assert!(output.contains("Fdt {"));
    assert!(output.contains("header:"));
    assert!(output.contains("nodes:"));
    // 验证节点信息
    assert!(output.contains("[/]"));
    assert!(output.contains("address_cells="));
    assert!(output.contains("size_cells="));
    // 验证属性解析（reg 应该显示解析后的地址）
    assert!(output.contains("reg: ["));
    assert!(output.contains("addr:"));
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
            "node: {} (level={}, addr_cells={}, size_cells={}, parent_addr_cells={}, parent_size_cells={})",
            node.name(),
            node.level(),
            node.address_cells,
            node.size_cells,
            node.reg_address_cells(),
            node.reg_size_cells()
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
                Property::Reg(reg) => info!("  reg ({} bytes)", reg.as_slice().len()),
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

#[test]
fn test_reg_parsing() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    info!("=== Reg Parsing Test ===");
    for node in fdt.all_nodes() {
        if let Some(reg) = node.reg() {
            info!("node: {}", node.name());
            info!(
                "  address_cells={}, size_cells={}",
                node.reg_address_cells(),
                node.reg_size_cells()
            );

            // 测试 as_u32_iter
            let u32_values: Vec<_> = reg.as_u32_iter().collect();
            info!("  raw u32: {:x?}", u32_values);

            // 测试 RegInfo iter
            for reg_info in reg.iter() {
                info!(
                    "  RegInfo: address={:#x}, size={:?}",
                    reg_info.address, reg_info.size
                );
            }
        }
    }
}

#[test]
fn test_memory_node() {
    init_logging();

    // 测试 RPi 4B DTB
    info!("=== Testing RPi 4B DTB ===");
    let raw = fdt_rpi_4b();
    test_memory_in_fdt(&raw, "RPi 4B");

    // 测试 QEMU DTB
    info!("\n=== Testing QEMU DTB ===");
    let raw = fdt_qemu();
    test_memory_in_fdt(&raw, "QEMU");
}

fn test_memory_in_fdt(raw: &[u8], name: &str) {
    let fdt = Fdt::from_bytes(raw).unwrap();

    let mut memory_found = false;
    for node in fdt.all_nodes() {
        if node.name().starts_with("memory@") || node.name() == "memory" {
            memory_found = true;
            info!(
                "[{}] Found memory node: {} (level={})",
                name,
                node.name(),
                node.level()
            );
            info!(
                "[{}]   parent_address_cells={}, parent_size_cells={}",
                name, node.context.parent_address_cells, node.context.parent_size_cells
            );

            // 解析 reg 属性
            if let Some(reg) = node.reg() {
                info!("[{}]   reg property found:", name);
                info!(
                    "[{}]     address_cells={}, size_cells={}",
                    name,
                    node.reg_address_cells(),
                    node.reg_size_cells()
                );
                // 打印原始数据
                let raw_data = reg.as_slice();
                info!(
                    "[{}]     raw data ({} bytes): {:02x?}",
                    name,
                    raw_data.len(),
                    raw_data
                );
                // 打印 u32 值
                let u32_values: Vec<_> = reg.as_u32_iter().collect();
                info!("[{}]     u32 values: {:x?}", name, u32_values);

                for (i, reg_info) in reg.iter().enumerate() {
                    info!(
                        "[{}]     reg[{}]: address={:#x}, size={:?}",
                        name, i, reg_info.address, reg_info.size
                    );
                }
            } else {
                info!("[{}]   No reg property found", name);
            }

            // 打印所有属性
            info!("[{}]   All properties:", name);
            for prop in node.properties() {
                info!("[{}]     {}", name, prop.name());
            }
        }
    }

    assert!(memory_found, "{}: memory node not found", name);
}
