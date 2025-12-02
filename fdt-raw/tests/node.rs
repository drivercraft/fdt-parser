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
    assert!(output.contains("child:"));
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
                    "  RegInfo: child_bus_addr={:#x}, address={:#x}, size={:?}",
                    reg_info.child_bus_address, reg_info.address, reg_info.size
                );
            }
        }
    }
}

#[test]
fn test_rpi4b_ranges() {
    init_logging();
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    info!("=== RPi 4B Ranges Test ===");

    // 收集需要验证的节点信息
    let mut serial_7e201000_found = false;
    let mut mmc_7e202000_found = false;
    let mut local_intc_found = false;
    let mut gic_found = false;
    let mut v3d_found = false;
    let mut pcie_found = false;
    let mut usb_xhci_found = false;

    for node in fdt.all_nodes() {
        let ranges_count = node.context.ranges.len();
        if ranges_count > 0 || node.name().contains("soc") || node.name().contains("pci") {
            info!(
                "node: {} (level={}, ranges_count={})",
                node.name(),
                node.level(),
                ranges_count
            );

            // 打印 ranges 条目
            for (i, range) in node.context.ranges.iter().enumerate() {
                info!(
                    "  range[{}]: child_bus={:#x}, parent_bus={:#x}, length={:#x}",
                    i, range.child_bus_addr, range.parent_bus_addr, range.length
                );
            }

            // 如果有 reg 属性，解析并显示地址转换
            if let Some(reg) = node.reg() {
                info!(
                    "  reg: address_cells={}, size_cells={}",
                    node.reg_address_cells(),
                    node.reg_size_cells()
                );
                for reg_info in reg.iter() {
                    info!(
                        "    child_bus_addr={:#x} -> address={:#x}, size={:?}",
                        reg_info.child_bus_address, reg_info.address, reg_info.size
                    );
                }
            }
        }

        // 验证 serial@7e201000: 0x7e201000 -> 0xfe201000 (通过 range 0x7e000000 -> 0xfe000000)
        if node.name() == "serial@7e201000" {
            serial_7e201000_found = true;
            assert_eq!(
                node.context.ranges.len(),
                3,
                "serial@7e201000 should have 3 ranges"
            );
            // 验证第一个 range
            assert_eq!(node.context.ranges[0].child_bus_addr, 0x7e000000);
            assert_eq!(node.context.ranges[0].parent_bus_addr, 0xfe000000);
            assert_eq!(node.context.ranges[0].length, 0x1800000);

            if let Some(reg) = node.reg() {
                let reg_info = reg.iter().next().unwrap();
                assert_eq!(reg_info.child_bus_address, 0x7e201000);
                assert_eq!(
                    reg_info.address, 0xfe201000,
                    "serial address translation failed"
                );
                assert_eq!(reg_info.size, Some(512));
            }
        }

        // 验证 mmc@7e202000: 0x7e202000 -> 0xfe202000
        if node.name() == "mmc@7e202000" {
            mmc_7e202000_found = true;
            if let Some(reg) = node.reg() {
                let reg_info = reg.iter().next().unwrap();
                assert_eq!(reg_info.child_bus_address, 0x7e202000);
                assert_eq!(
                    reg_info.address, 0xfe202000,
                    "mmc address translation failed"
                );
                assert_eq!(reg_info.size, Some(256));
            }
        }

        // 验证 local_intc@40000000: 0x40000000 -> 0xff800000 (通过 range 0x40000000 -> 0xff800000)
        if node.name() == "local_intc@40000000" {
            local_intc_found = true;
            if let Some(reg) = node.reg() {
                let reg_info = reg.iter().next().unwrap();
                assert_eq!(reg_info.child_bus_address, 0x40000000);
                assert_eq!(
                    reg_info.address, 0xff800000,
                    "local_intc address translation failed"
                );
                assert_eq!(reg_info.size, Some(256));
            }
        }

        // 验证 interrupt-controller@40041000 (GIC): 0x40041000 -> 0xff841000
        if node.name() == "interrupt-controller@40041000" {
            gic_found = true;
            if let Some(reg) = node.reg() {
                let regs: Vec<_> = reg.iter().collect();
                assert!(regs.len() >= 4, "GIC should have at least 4 reg entries");
                // 第一个 reg: 0x40041000 -> 0xff841000
                assert_eq!(regs[0].child_bus_address, 0x40041000);
                assert_eq!(
                    regs[0].address, 0xff841000,
                    "GIC distributor address translation failed"
                );
                assert_eq!(regs[0].size, Some(4096));
                // 第二个 reg: 0x40042000 -> 0xff842000
                assert_eq!(regs[1].child_bus_address, 0x40042000);
                assert_eq!(regs[1].address, 0xff842000);
            }
        }

        // 验证 v3d@7ec04000: 使用不同的 ranges
        if node.name() == "v3d@7ec04000" {
            v3d_found = true;
            assert_eq!(node.context.ranges.len(), 2, "v3d should have 2 ranges");
            // 验证 v3d 的特殊 ranges
            assert_eq!(node.context.ranges[0].child_bus_addr, 0x7c500000);
            assert_eq!(node.context.ranges[0].parent_bus_addr, 0xfc500000);

            if let Some(reg) = node.reg() {
                let regs: Vec<_> = reg.iter().collect();
                assert_eq!(regs.len(), 2);
                // 0x7ec00000 通过 range 0x7c500000->0xfc500000 映射
                // 偏移 = 0x7ec00000 - 0x7c500000 = 0x700000
                // 结果 = 0xfc500000 + 0x700000 = 0xfcc00000? 不对
                // 实际上应该是 0xfec00000，说明用了别的映射规则
                assert_eq!(regs[0].child_bus_address, 0x7ec00000);
                assert_eq!(
                    regs[0].address, 0xfec00000,
                    "v3d reg[0] address translation failed"
                );
            }
        }

        // 验证 pcie@7d500000
        if node.name() == "pcie@7d500000" {
            pcie_found = true;
            assert_eq!(node.context.ranges.len(), 4, "pcie should have 4 ranges");
            if let Some(reg) = node.reg() {
                let reg_info = reg.iter().next().unwrap();
                assert_eq!(reg_info.child_bus_address, 0x7d500000);
                assert_eq!(
                    reg_info.address, 0xfd500000,
                    "pcie address translation failed"
                );
            }
        }

        // 验证 xhci@7e9c0000
        if node.name() == "xhci@7e9c0000" {
            usb_xhci_found = true;
            if let Some(reg) = node.reg() {
                let reg_info = reg.iter().next().unwrap();
                assert_eq!(reg_info.child_bus_address, 0x7e9c0000);
                assert_eq!(
                    reg_info.address, 0xfe9c0000,
                    "xhci address translation failed"
                );
                assert_eq!(reg_info.size, Some(1048576)); // 1MB
            }
        }
    }

    // 确保所有关键节点都被找到并验证
    assert!(serial_7e201000_found, "serial@7e201000 not found");
    assert!(mmc_7e202000_found, "mmc@7e202000 not found");
    assert!(local_intc_found, "local_intc@40000000 not found");
    assert!(gic_found, "interrupt-controller@40041000 not found");
    assert!(v3d_found, "v3d@7ec04000 not found");
    assert!(pcie_found, "pcie@7d500000 not found");
    assert!(usb_xhci_found, "xhci@7e9c0000 not found");

    info!("=== All RPi 4B ranges assertions passed! ===");
}
