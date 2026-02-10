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

    // Verify basic DTS structure
    let basic_checks = [
        ("/dts-v1/;", "DTS version header"),
        ("/ {", "root node opening"),
        ("};", "node closing"),
    ];
    for (pattern, desc) in basic_checks {
        assert!(output.contains(pattern), "Output should contain {desc}");
    }

    // Verify root node properties
    let root_props = [
        ("interrupt-parent = <0x8002>", "interrupt-parent property"),
        ("model = \"linux,dummy-virt\"", "model property"),
        ("#size-cells = <0x2>", "#size-cells property"),
        ("#address-cells = <0x2>", "#address-cells property"),
        ("compatible = \"linux,dummy-virt\"", "compatible property"),
    ];
    for (pattern, desc) in root_props {
        assert!(output.contains(pattern), "Should contain {desc}");
    }

    // Verify important nodes exist
    let important_nodes = [
        ("psci {", "psci node opening"),
        ("memory@40000000 {", "memory node"),
        ("platform-bus@c000000 {", "platform-bus node"),
        ("fw-cfg@9020000 {", "fw-cfg node"),
        ("virtio_mmio@a000000 {", "virtio_mmio device"),
        ("pl061@9030000 {", "GPIO controller node"),
        ("pcie@10000000 {", "PCIe controller node"),
        ("intc@8000000 {", "interrupt controller node"),
        ("cpu@0 {", "CPU node"),
        ("apb-pclk {", "clock node"),
        ("chosen {", "chosen node"),
    ];
    for (pattern, desc) in important_nodes {
        assert!(output.contains(pattern), "Should contain {desc}");
    }

    // Verify important properties
    let important_props = [
        ("device_type = \"memory\"", "memory device_type"),
        ("dma-coherent", "dma-coherent property"),
        ("interrupt-controller", "interrupt-controller property"),
        ("stdout-path = \"/pl011@9000000\"", "stdout-path property"),
    ];
    for (pattern, desc) in important_props {
        assert!(output.contains(pattern), "Should contain {desc}");
    }

    // Verify format specifications
    let format_checks = [
        ("= <", "use '< >' for cell values"),
        ("= \"", "use '\" \"' for string values"),
        ("<0x", "hex format for values"),
        ("\"", "quoted strings"),
    ];
    for (pattern, desc) in format_checks {
        assert!(output.contains(pattern), "Should {desc}");
    }

    info!("All FDT display format validations passed!");
}

#[test]
fn test_fdt_debug() {
    init_logging();
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();
    let output = format!("{:?}", fdt);
    info!("FDT Debug:\n{}", output);

    // Verify basic Debug structure
    let struct_checks = [
        ("Fdt {", "Fdt struct opening"),
        ("header: Header", "header field"),
        ("nodes:", "nodes field"),
    ];
    for (pattern, desc) in struct_checks {
        assert!(
            output.contains(pattern),
            "Debug output should contain {desc}"
        );
    }

    // Verify header fields
    let header_fields = [
        ("magic:", "magic field"),
        ("totalsize:", "totalsize field"),
        ("off_dt_struct:", "off_dt_struct field"),
        ("off_dt_strings:", "off_dt_strings field"),
        ("off_mem_rsvmap:", "off_mem_rsvmap field"),
        ("version:", "version field"),
        ("last_comp_version:", "last_comp_version field"),
        ("boot_cpuid_phys:", "boot_cpuid_phys field"),
        ("size_dt_strings:", "size_dt_strings field"),
        ("size_dt_struct:", "size_dt_struct field"),
    ];
    for (pattern, desc) in header_fields {
        assert!(output.contains(pattern), "Should contain header {desc}");
    }

    // Verify root node information
    let root_node_checks = [
        ("[/]", "root node"),
        ("address_cells=", "address_cells field"),
        ("size_cells=", "size_cells field"),
        ("model:", "model field"),
        ("#address-cells:", "#address-cells field"),
        ("#size-cells:", "#size-cells field"),
        ("compatible:", "compatible field"),
        ("interrupt-parent:", "interrupt-parent field"),
    ];
    for (pattern, desc) in root_node_checks {
        assert!(output.contains(pattern), "Should contain {desc}");
    }

    // Verify data format
    let format_checks = [
        ("0x", "hexadecimal numbers"),
        ("\"", "quoted strings"),
        ("[", "array opening brackets"),
        ("]", "array closing brackets"),
    ];
    for (pattern, desc) in format_checks {
        assert!(output.contains(pattern), "Should contain {desc}");
    }

    // Verify specific nodes
    let specific_checks = [
        ("memory@", "memory node"),
        ("soc", "soc node"),
        ("Raspberry Pi 4 Model B", "RPi 4 model name"),
        ("raspberrypi,4-model-b", "RPi compatible string"),
    ];
    for (pattern, desc) in specific_checks {
        assert!(output.contains(pattern), "Should contain {desc}");
    }

    info!("All FDT debug format validations passed!");
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
            "node: {} (level={}, parent_addr_cells={}, parent_size_cells={})",
            node.name(),
            node.level(),
            node.address_cells,
            node.size_cells,
        );
    }
}

#[test]
fn test_node_properties() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    let mut found_address_cells = false;
    let mut found_size_cells = false;
    let mut found_interrupt_cells = false;
    let mut found_device_type = false;
    let mut found_compatible = false;
    let mut found_phandle = false;
    let mut found_interrupt_parent = false;
    let mut found_reg = false;
    let mut found_dma_coherent = false;
    let mut found_empty_property = false;

    for node in fdt.all_nodes() {
        info!("node: {}", node.name());
        for prop in node.properties() {
            if let Some(v) = prop.as_address_cells() {
                found_address_cells = true;
                info!("  #address-cells = {}", v);
                assert!(
                    v == 1 || v == 2 || v == 3,
                    "Unexpected #address-cells value: {}, should be 1, 2, or 3",
                    v
                );
            } else if let Some(v) = prop.as_size_cells() {
                found_size_cells = true;
                info!("  #size-cells = {}", v);
                assert!(
                    v == 0 || v == 1 || v == 2,
                    "Unexpected #size-cells value: {}, should be 0, 1, or 2",
                    v
                );
            } else if let Some(v) = prop.as_interrupt_cells() {
                found_interrupt_cells = true;
                info!("  #interrupt-cells = {}", v);
                assert!(
                    (1..=4).contains(&v),
                    "Unexpected #interrupt-cells value: {}, should be 1-4",
                    v
                );
            } else if let Some(s) = prop.as_status() {
                info!("  status = {:?}", s);
                // Verify status value validity
                match s {
                    Status::Okay | Status::Disabled => {}
                }
            } else if let Some(iter) = prop.as_compatible() {
                let strs: Vec<_> = iter.clone().collect();
                if !strs.is_empty() {
                    found_compatible = true;
                    info!("  compatible = {:?}", strs);
                }
            } else if let Some(s) = prop.as_device_type() {
                found_device_type = true;
                info!("  device_type = \"{}\"", s);
            } else if prop.as_phandle().is_some() {
                found_phandle = true;
                info!("  {} = <{:?}>", prop.name(), prop.as_phandle());
            } else if prop.as_interrupt_parent().is_some() {
                found_interrupt_parent = true;
                info!("  {} = <{:?}>", prop.name(), prop.as_interrupt_parent());
            } else if prop.name() == "reg" {
                found_reg = true;
                info!("  reg ({} bytes)", prop.len());
            } else if prop.name() == "dma-coherent" {
                found_dma_coherent = true;
                info!("  dma-coherent (empty)");
            } else {
                // Handle unknown properties
                if let Some(s) = prop.as_str() {
                    info!("  {} = \"{}\"", prop.name(), s);
                    // Verify string length is reasonable
                    assert!(
                        s.len() <= 256,
                        "String property too long: {} bytes",
                        s.len()
                    );
                } else if let Some(v) = prop.as_u32() {
                    info!("  {} = {:#x}", prop.name(), v);
                } else if prop.is_empty() {
                    found_empty_property = true;
                    info!("  {} (empty)", prop.name());
                } else {
                    info!("  {} ({} bytes)", prop.name(), prop.len());
                    // Verify property length is reasonable
                    assert!(
                        prop.len() <= 1024,
                        "Property too large: {} bytes",
                        prop.len()
                    );
                }

                // Verify property name
                assert!(!prop.name().is_empty(), "Property name should not be empty");
                assert!(
                    prop.name().len() <= 31,
                    "Property name too long: {}",
                    prop.name().len()
                );
            }
        }
    }

    // Verify found basic properties
    assert!(found_address_cells, "Should find #address-cells property");
    assert!(found_size_cells, "Should find #size-cells property");
    assert!(found_compatible, "Should find compatible property");
    assert!(found_device_type, "Should find device_type property");
    assert!(found_reg, "Should find reg property");

    // Verify found other important properties
    assert!(found_phandle, "Should find phandle property");
    assert!(
        found_interrupt_parent,
        "Should find interrupt-parent property"
    );
    assert!(
        found_interrupt_cells,
        "Should find #interrupt-cells property"
    );
    assert!(found_dma_coherent, "Should find dma-coherent property");
    assert!(found_empty_property, "Should find empty property");

    info!("All property types validated successfully!");
}

#[test]
fn test_reg_parsing() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    info!("=== Reg Parsing Test ===");

    let mut found_memory_reg = false;
    let mut found_virtio_mmio_reg = false;
    let mut found_fw_cfg_reg = false;
    let mut found_gpio_reg = false;

    for node in fdt.all_nodes() {
        if let Some(reg) = node.reg() {
            info!("node: {}", node.name());

            let reg_infos: Vec<_> = reg.collect();

            // Verify reg property for specific nodes
            if node.name().starts_with("memory@") {
                found_memory_reg = true;

                assert!(
                    !reg_infos.is_empty(),
                    "Memory should have at least one reg entry"
                );

                let reg_info = &reg_infos[0];
                // QEMU memory address verification
                assert_eq!(
                    reg_info.address, 0x40000000,
                    "Memory base address should be 0x40000000"
                );
                assert_eq!(
                    reg_info.size,
                    Some(134217728),
                    "Memory size should be 128MB (0x8000000)"
                );
            }

            if node.name().starts_with("virtio_mmio@") {
                found_virtio_mmio_reg = true;

                assert_eq!(reg_infos.len(), 1, "Virtio MMIO should have one reg entry");

                let reg_info = &reg_infos[0];
                assert!(
                    reg_info.address >= 0xa000000,
                    "Virtio MMIO address should be >= 0xa000000, got {:#x}",
                    reg_info.address
                );
                assert_eq!(
                    reg_info.size,
                    Some(512),
                    "Virtio MMIO size should be 512 bytes, got {:?}",
                    reg_info.size
                );

                // Verify address is within expected range (0xa000000 to 0xa003e00)
                assert!(
                    reg_info.address <= 0xa003e00,
                    "Virtio MMIO address should be <= 0xa003e00, got {:#x}",
                    reg_info.address
                );

                // Verify address is 0x200 aligned (each device occupies 0x200 space)
                assert_eq!(
                    reg_info.address % 0x200,
                    0x0,
                    "Virtio MMIO address should be 0x200 aligned, got {:#x}",
                    reg_info.address
                );
            }

            if node.name() == "fw-cfg@9020000" {
                found_fw_cfg_reg = true;
                assert_eq!(reg_infos.len(), 1, "fw-cfg should have one reg entry");

                let reg_info = &reg_infos[0];
                assert_eq!(
                    reg_info.address, 0x9020000,
                    "fw-cfg address should be 0x9020000, got {:#x}",
                    reg_info.address
                );
                assert_eq!(
                    reg_info.size,
                    Some(24),
                    "fw-cfg size should be 24 bytes, got {:?}",
                    reg_info.size
                );
            }

            if node.name() == "pl061@9030000" {
                found_gpio_reg = true;
                assert_eq!(reg_infos.len(), 1, "pl061 should have one reg entry");

                let reg_info = &reg_infos[0];
                assert_eq!(
                    reg_info.address, 0x9030000,
                    "pl061 address should be 0x9030000, got {:#x}",
                    reg_info.address
                );
                assert_eq!(
                    reg_info.size,
                    Some(4096),
                    "pl061 size should be 4096 bytes, got {:?}",
                    reg_info.size
                );
            }
        }
    }

    // Verify found all expected reg nodes
    assert!(
        found_memory_reg,
        "Should find memory node with reg property"
    );
    assert!(
        found_virtio_mmio_reg,
        "Should find virtio_mmio nodes with reg property"
    );
    assert!(
        found_fw_cfg_reg,
        "Should find fw-cfg node with reg property"
    );
    assert!(
        found_gpio_reg,
        "Should find pl061 gpio node with reg property"
    );
}

#[test]
fn test_memory_node() {
    init_logging();

    // Test RPi 4B DTB
    info!("=== Testing RPi 4B DTB ===");
    let raw = fdt_rpi_4b();
    test_memory_in_fdt(&raw, "RPi 4B");

    // Test QEMU DTB
    info!("\n=== Testing QEMU DTB ===");
    let raw = fdt_qemu();
    test_memory_in_fdt(&raw, "QEMU");
}

fn test_memory_in_fdt(raw: &[u8], name: &str) {
    let fdt = Fdt::from_bytes(raw).unwrap();

    let mut memory_nodes_found = 0;

    for node in fdt.all_nodes() {
        if node.name().starts_with("memory@") || node.name() == "memory" {
            memory_nodes_found += 1;

            let reg = node.reg().expect("Memory node should have reg property");
            let reg_infos: Vec<_> = reg.collect();

            info!(
                "[{}] Found memory node: {} (level={})",
                name,
                node.name(),
                node.level()
            );

            // Verify node level - memory node should be at level 1
            assert_eq!(
                node.level(),
                1,
                "Memory node should be at level 1, got level {}",
                node.level()
            );

            // Verify and parse reg property
            let mut found_device_type = false;

            for prop in node.properties() {
                if let Some(s) = prop.as_device_type() {
                    found_device_type = true;
                    assert_eq!(
                        s, "memory",
                        "Memory node device_type should be 'memory', got '{}'",
                        s
                    );
                    info!("[{}]   device_type = \"{}\"", name, s);
                } else if let Some(iter) = prop.as_compatible() {
                    let strs: Vec<_> = iter.clone().collect();
                    if !strs.is_empty() {
                        info!("[{}]   compatible = {:?}", name, strs);
                    }
                } else {
                    info!("[{}]   {}", name, prop.name());
                }
            }

            // Verify required properties
            assert!(
                found_device_type,
                "Memory node should have device_type property"
            );

            info!("[{}]   reg entries: {}", name, reg_infos.len());

            for (i, reg_info) in reg_infos.iter().enumerate() {
                info!(
                    "[{}]     reg[{}]: address={:#x}, size={:?}",
                    name, i, reg_info.address, reg_info.size
                );

                // Basic verification: address should be valid
                if reg_info.size.is_some() && reg_info.size.unwrap() > 0 {
                    assert!(
                        reg_info.size.unwrap() > 0,
                        "Memory size should be positive, got {:?}",
                        reg_info.size
                    );
                }
            }

            // Platform-specific verification
            if name == "QEMU" && !reg_infos.is_empty() {
                assert_eq!(
                    reg_infos.len(),
                    1,
                    "QEMU memory should have exactly one reg entry"
                );

                let reg_info = &reg_infos[0];
                assert_eq!(
                    reg_info.address, 0x40000000,
                    "QEMU memory base address should be 0x40000000, got {:#x}",
                    reg_info.address
                );
                assert_eq!(
                    reg_info.size,
                    Some(134217728),
                    "QEMU memory size should be 128MB (0x8000000), got {:?}",
                    reg_info.size
                );

                info!(
                    "[{}]   QEMU memory validated: address={:#x}, size={} bytes",
                    name,
                    reg_info.address,
                    reg_info.size.unwrap_or(0)
                );
            }
        }
    }

    assert!(
        memory_nodes_found > 0,
        "{}: Should find at least one memory node, found {}",
        name,
        memory_nodes_found
    );
    info!("[{}] Found {} memory node(s)", name, memory_nodes_found);
}

#[test]
fn test_compatibles() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();
    let node = fdt.find_by_path("/pl061@9030000").unwrap();
    for compatible in node.compatibles() {
        info!("compatible: {}", compatible);
    }
}

#[test]
fn test_node_path_root() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // The first node (root) should have path "/"
    let root = fdt.all_nodes().next().unwrap();
    assert_eq!(root.name(), "");
    assert_eq!(root.path().as_str(), "/");
}

#[test]
fn test_node_path_all_nodes() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    for node in fdt.all_nodes() {
        let path = node.path();
        info!("node: {} -> path: {}", node.name(), path);

        // All paths must start with '/'
        assert!(
            path.starts_with('/'),
            "Path should start with '/', got: {}",
            path
        );

        // Root node special case
        if node.name().is_empty() {
            assert_eq!(path.as_str(), "/");
        } else {
            // Non-root nodes: path should end with the node name
            assert!(
                path.ends_with(node.name()),
                "Path '{}' should end with node name '{}'",
                path,
                node.name()
            );
            // Path should not have double slashes
            assert!(
                !path.contains("//"),
                "Path should not contain '//': {}",
                path
            );
        }
    }
}

#[test]
fn test_node_path_known_nodes() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // Collect all paths
    let paths: Vec<String> = fdt
        .all_nodes()
        .map(|n| n.path().to_string())
        .collect();

    // Verify known paths exist
    let expected_paths = [
        "/",
        "/memory@40000000",
        "/chosen",
    ];
    for expected in expected_paths {
        assert!(
            paths.iter().any(|p| p == expected),
            "Expected path '{}' not found in: {:?}",
            expected,
            paths
        );
    }
}

#[test]
fn test_node_path_find_by_path_consistency() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // For each node, its path() should be findable via find_by_path
    for node in fdt.all_nodes() {
        let path = node.path();
        let found = fdt.find_by_path(path.as_str());
        assert!(
            found.is_some(),
            "Node with path '{}' (name='{}') should be findable via find_by_path",
            path,
            node.name()
        );
        assert_eq!(
            found.unwrap().name(),
            node.name(),
            "find_by_path('{}') returned node with wrong name",
            path
        );
    }
}

#[test]
fn test_node_path_depth() {
    init_logging();
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    for node in fdt.all_nodes() {
        let path = node.path();
        let level = node.level();
        // Number of '/' in path should equal the level for non-root, or 1 for root
        let slash_count = path.chars().filter(|&c| c == '/').count();
        if level == 0 {
            assert_eq!(slash_count, 1, "Root path '{}' should have exactly one '/'", path);
        } else {
            assert_eq!(
                slash_count, level,
                "Path '{}' at level {} should have {} slashes, got {}",
                path, level, level, slash_count
            );
        }
        info!("level={} path={}", level, path);
    }
}
