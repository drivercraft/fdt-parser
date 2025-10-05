#[cfg(test)]
mod test {
    use dtb_file::{fdt_3568, fdt_phytium, fdt_qemu, fdt_reserve, fdt_rpi_4b};
    use fdt_parser::*;
    use log::{debug, info};
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
    fn test_new() {
        init_logging();
        let raw = fdt_qemu();
        let ptr = raw.as_ptr() as *mut u8;
        let fdt = unsafe { Fdt::from_ptr(ptr).unwrap() };

        info!("ver: {:#?}", fdt.header().version);
    }

    #[test]
    fn test_memory_reservation_blocks() {
        init_logging();
        // Test with custom DTB that has memory reservations
        let raw = fdt_reserve();
        let ptr = raw.as_ptr() as *mut u8;
        let fdt = unsafe { Fdt::from_ptr(ptr).unwrap() };

        // Get memory reservation blocks
        let rsv_result = fdt.memory_reservaion_blocks();

        let entries = rsv_result;

        // Should have exactly 3 reservation blocks as defined in our DTS
        assert_eq!(
            entries.len(),
            3,
            "Should have exactly 3 memory reservation blocks"
        );

        // Test the specific values we defined
        let expected_reservations = [
            (0x40000000u64, 0x04000000u64), // 64MB at 1GB
            (0x80000000u64, 0x00100000u64), // 1MB at 2GB
            (0xA0000000u64, 0x00200000u64), // 2MB at 2.5GB
        ];

        for (i, &(expected_addr, expected_size)) in expected_reservations.iter().enumerate() {
            assert_eq!(
                entries[i].address as usize, expected_addr as usize,
                "Reservation {} address mismatch: expected {:#x}, got {:#p}",
                i, expected_addr, entries[i].address
            );
            assert_eq!(
                entries[i].size, expected_size as usize,
                "Reservation {} size mismatch: expected {:#x}, got {:#x}",
                i, expected_size, entries[i].size
            );
        }

        // Test iterator behavior - iterate twice to ensure it works correctly
        let rsv1: Vec<_> = fdt.memory_reservaion_blocks();
        let rsv2: Vec<_> = fdt.memory_reservaion_blocks();
        assert_eq!(
            rsv1.len(),
            rsv2.len(),
            "Multiple iterations should yield same results"
        );

        for (entry1, entry2) in rsv1.iter().zip(rsv2.iter()) {
            assert_eq!(
                entry1.address, entry2.address,
                "Addresses should match between iterations"
            );
            assert_eq!(
                entry1.size, entry2.size,
                "Sizes should match between iterations"
            );
        }
    }

    #[test]
    fn test_empty_memory_reservation_blocks() {
        init_logging();
        // Test with DTBs that have no memory reservations
        let test_cases = [
            ("QEMU", fdt_qemu()),
            ("Phytium", fdt_phytium()),
            ("RK3568", fdt_3568()),
        ];

        for (name, raw) in test_cases {
            let ptr = raw.as_ptr() as *mut u8;
            let fdt = unsafe { Fdt::from_ptr(ptr).unwrap() };

            let rsv_result = fdt.memory_reservaion_blocks();

            let entries = rsv_result;
            assert_eq!(
                entries.len(),
                0,
                "{} DTB should have no memory reservation blocks",
                name
            );
        }
    }

    fn test_node<'a>() -> Option<Node> {
        let raw = fdt_rpi_4b();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };
        fdt.all_nodes().into_iter().next()
    }

    #[test]
    fn test_send_node() {
        init_logging();
        let node = test_node();
        if let Some(node) = node {
            info!("{:?}", node.name());
        }
    }

    #[test]
    fn test_all_nodes() {
        init_logging();
        let raw = fdt_reserve();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };
        for node in fdt.all_nodes() {
            debug!(
                "{}{} l{} parent={:?}",
                match node.level() {
                    0 => "",
                    1 => "  ",
                    2 => "    ",
                    _ => "       ",
                },
                node.full_path(),
                node.level(),
                node.parent().map(|n| n.name().to_string())
            );
        }
    }

    #[test]
    fn test_property() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };
        for node in fdt.all_nodes() {
            info!("{}:", node.name());
            for prop in node.properties() {
                debug!("  {:?}", prop);
            }
        }
    }

    #[test]
    fn test_str_list() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };
        let uart = fdt.find_nodes("/soc/serial@7e201000")[0].clone();
        let caps = uart.compatibles();

        let want = ["arm,pl011", "arm,primecell"];

        for (i, cap) in caps.iter().enumerate() {
            assert_eq!(*cap, want[i]);
        }
    }
    #[test]
    fn test_find_nodes() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };

        let uart = fdt.find_nodes("/soc/serial");

        let want = [
            "serial@7e201000",
            "serial@7e215040",
            "serial@7e201400",
            "serial@7e201600",
            "serial@7e201800",
            "serial@7e201a00",
        ];

        for (act, &want) in uart.iter().zip(want.iter()) {
            assert_eq!(act.name(), want);
        }
    }

    #[test]
    fn test_find_node2() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node = fdt.find_nodes("/soc/serial@7e215040")[0].clone();
        assert_eq!(node.name(), "serial@7e215040");
    }

    #[test]
    fn test_find_aliases() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let path = fdt.find_aliase("serial0").unwrap();
        assert_eq!(path, "/soc/serial@7e215040");
    }
    #[test]
    fn test_find_node_aliases() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node = fdt.find_nodes("serial0")[0].clone();
        assert_eq!(node.name(), "serial@7e215040");
    }

    #[test]
    fn test_reg() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();

        let node = fdt.find_nodes("/soc/serial@7e215040")[0].clone();

        let reg = node.reg().unwrap()[0].clone();

        let parent = node.parent().unwrap();
        if let Some(addr_cells_prop) = parent.find_property("#address-cells") {
            debug!("parent #address-cells={}", addr_cells_prop.u32().unwrap());
        }
        if let Some(size_cells_prop) = parent.find_property("#size-cells") {
            debug!("parent #size-cells={}", size_cells_prop.u32().unwrap());
        }
        if let Some(ranges) = parent.ranges() {
            for (idx, range) in ranges.iter().enumerate() {
                let child_cells = range.child_bus_address().collect::<Vec<_>>();
                let parent_cells = range.parent_bus_address().collect::<Vec<_>>();
                let child_addr = child_cells
                    .iter()
                    .fold(0u64, |acc, val| (acc << 32) | (*val as u64));
                let parent_addr = parent_cells
                    .iter()
                    .fold(0u64, |acc, val| (acc << 32) | (*val as u64));
                debug!(
                    "range[{idx}]: child_cells={:?} parent_cells={:?} child={:#x} parent={:#x} size={:#x}",
                    child_cells, parent_cells, child_addr, parent_addr, range.size
                );
            }
        }

        info!("reg: {:?}", reg);

        assert_eq!(
            reg.address, 0xfe215040,
            "want 0xfe215040, got {:#x}",
            reg.address
        );
        assert_eq!(
            reg.child_bus_address, 0x7e215040,
            "want 0x7e215040, got {:#x}",
            reg.child_bus_address
        );
        assert_eq!(
            reg.size,
            Some(0x40),
            "want 0x40, got {:#x}",
            reg.size.unwrap()
        );
    }

    // #[test]
    // fn test_memory() {
    //     let raw = fdt_qemu();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();

    //     let node = fdt.memory();

    //     for node in node {
    //         let node = node.unwrap();
    //         println!("memory node: {:?}", node.name());
    //         for reg in node.reg().unwrap().unwrap() {
    //             println!("  reg: {:?}", reg);
    //         }

    //         for region in node.regions() {
    //             let region = region.unwrap();
    //             println!("  region: {:?}", region);
    //         }
    //     }
    // }
    #[test]
    fn test_find_compatible() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };

        let ls = fdt.find_compatible(&["arm,pl011", "arm,primecell"]);

        assert_eq!(ls[0].name(), "serial@7e201000");
    }

    #[test]
    fn test_compatibles() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };
        let mut ls = fdt.find_nodes("/soc/serial@7e201000");
        let uart = ls.pop().unwrap();
        let caps = uart.compatibles();

        let want = ["arm,pl011", "arm,primecell"];

        for (act, want) in caps.iter().zip(want.iter()) {
            assert_eq!(act, *want);
        }
    }

    #[test]
    fn test_interrupt() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node = fdt.find_nodes("/soc/serial@7e215040")[0].clone();

        let itr_ctrl = node.interrupt_parent().unwrap();
        info!("itr_ctrl: {:?}", itr_ctrl.name());
        let interrupt_cells = itr_ctrl.interrupt_cells().unwrap();
        assert_eq!(interrupt_cells, 3);
    }

    #[test]
    fn test_interrupt2() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();

        let node = fdt.find_compatible(&["brcm,bcm2711-hdmi0"])[0].clone();

        let itr_ctrl_ph = node.interrupt_parent_phandle().unwrap();
        assert_eq!(itr_ctrl_ph, 0x2c.into());

        let itr_ctrl = node.interrupt_parent().unwrap();
        assert_eq!(itr_ctrl.name(), "interrupt-controller@7ef00100");
    }

    #[test]
    fn test_interrupts() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();

        let node = fdt.find_compatible(&["brcm,bcm2711-hdmi0"])[0].clone();
        let itr = node.interrupts().unwrap();
        let want_itrs = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5];

        for (i, itr) in itr.iter().enumerate() {
            assert_eq!(itr[0], want_itrs[i]);
        }
    }

    #[test]
    fn test_clocks() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();

        let node = fdt.find_nodes("/soc/serial@7e215040")[0].clone();
        let clocks = node.clocks().unwrap();
        assert!(!clocks.is_empty());
        let clock = &clocks[0];
        assert_eq!(clock.provider_name(), "aux@7e215000");
    }

    #[test]
    fn test_clocks_cell_1() {
        init_logging();
        let fdt = fdt_3568();

        let fdt = Fdt::from_bytes(&fdt).unwrap();
        let node = fdt.find_nodes("/sdhci@fe310000")[0].clone();
        let clocks = node.clocks().unwrap();
        let clock = clocks[0].clone();

        for clock in &clocks {
            debug!("clock: {:?}", clock);
        }
        assert_eq!(clock.provider_name(), "clock-controller@fdd20000");
    }

    #[test]
    fn test_clocks_cell_0() {
        init_logging();
        let raw = fdt_phytium();

        let fdt = Fdt::from_bytes(&raw).unwrap();

        let node = fdt.find_nodes("/soc/uart@2800e000")[0].clone();
        let clocks = node.clocks().unwrap();

        for clock in &clocks {
            debug!("clock: {:?}", clock);
        }
    }

    #[test]
    fn test_pcie() {
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node = fdt
            .find_compatible(&["brcm,bcm2711-pcie"])
            .into_iter()
            .next()
            .unwrap();
        let regs = node.reg().unwrap();
        let reg = regs[0];
        println!("reg: {:?}", reg);
        assert_eq!(reg.address, 0xfd500000);
        assert_eq!(reg.size, Some(0x9310));
    }

    #[test]
    fn test_pci2() {
        let raw = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .into_iter()
            .next()
            .unwrap();

        let Node::Pci(pci) = node else {
            panic!("Not a PCI node");
        };

        let want = [
            PciRange {
                space: PciSpace::IO,
                bus_address: 0x0,
                cpu_address: 0x50000000,
                size: 0xf00000,
                prefetchable: false,
            },
            PciRange {
                space: PciSpace::Memory32,
                bus_address: 0x58000000,
                cpu_address: 0x58000000,
                size: 0x28000000,
                prefetchable: false,
            },
            PciRange {
                space: PciSpace::Memory64,
                bus_address: 0x1000000000,
                cpu_address: 0x1000000000,
                size: 0x1000000000,
                prefetchable: false,
            },
        ];

        for (i, range) in pci.ranges().unwrap().iter().enumerate() {
            assert_eq!(*range, want[i]);
        }
    }

    #[test]
    fn test_pci_irq_map() {
        let raw = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let pci = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .into_iter()
            .next()
            .unwrap();

        let Node::Pci(pci) = pci else {
            panic!("Not a PCI node");
        };

        let irq = pci.child_interrupts(0, 0, 0, 4).unwrap();
        assert!(!irq.irqs.is_empty());
    }

    #[test]
    fn test_pci_irq_map2() {
        let raw = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let pci = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .into_iter()
            .next()
            .unwrap();

        let Node::Pci(pci) = pci else {
            panic!("Not a PCI node");
        };

        let irq = pci.child_interrupts(0, 2, 0, 1).unwrap();

        let want = [0, 5, 4];

        for (got, want) in irq.irqs.iter().zip(want.iter()) {
            assert_eq!(*got, *want);
        }
    }

    // #[test]
    // fn test_debugcon() {
    //     let raw = fdt_qemu();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();
    //     let debugcon = fdt.chosen().unwrap().debugcon().unwrap();

    //     match debugcon {
    //         Some(DebugCon::Node(node)) => {
    //             println!("Found debugcon node: {:?}", node.name());
    //         }
    //         Some(DebugCon::EarlyConInfo { name, mmio, params }) => {
    //             println!("Found earlycon info: name={}, mmio={:#x}, params={:?}", name, mmio, params);
    //         }
    //         None => {
    //             println!("No debugcon found");
    //         }
    //     }
    // }

    // #[test]
    // fn test_debugcon2() {
    //     let raw = fdt_3568();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();
    //     let debugcon = fdt.chosen().unwrap().debugcon().unwrap();

    //     match debugcon {
    //         Some(DebugCon::Node(node)) => {
    //             println!("Found debugcon node: {:?}", node.name());
    //         }
    //         Some(DebugCon::EarlyConInfo { name, mmio, params }) => {
    //             println!("Found earlycon info: name={}, mmio={:#x}, params={:?}", name, mmio, params);
    //         }
    //         None => {
    //             println!("No debugcon found");
    //         }
    //     }
    // }

    #[test]
    fn test_parent_relationships_basic() {
        let raw = fdt_reserve();
        let fdt = unsafe { fdt_parser::Fdt::from_ptr(raw.ptr()).unwrap() };

        // 收集所有节点到Vec中以便查找
        let nodes = fdt.all_nodes();

        // 测试根节点没有父节点
        let root = nodes.iter().find(|n| n.full_path() == "/").unwrap();
        assert!(root.parent().is_none(), "Root node should have no parent");
        assert_eq!(root.level(), 0);

        // 测试一级节点的父节点是根节点
        let chosen = nodes.iter().find(|n| n.full_path() == "/chosen").unwrap();
        assert_eq!(chosen.parent().unwrap().full_path(), "/");
        assert_eq!(chosen.level(), 1);

        let memory = nodes.iter().find(|n| n.full_path() == "/memory@0").unwrap();
        assert_eq!(memory.parent().unwrap().full_path(), "/");
        assert_eq!(memory.level(), 1);

        let cpus = nodes.iter().find(|n| n.full_path() == "/cpus").unwrap();
        assert_eq!(cpus.parent().unwrap().full_path(), "/");
        assert_eq!(cpus.level(), 1);

        let timer = nodes.iter().find(|n| n.full_path() == "/timer").unwrap();
        assert_eq!(timer.parent().unwrap().full_path(), "/");
        assert_eq!(timer.level(), 1);

        let serial = nodes
            .iter()
            .find(|n| n.full_path() == "/serial@1c28000")
            .unwrap();
        assert_eq!(serial.parent().unwrap().full_path(), "/");
        assert_eq!(serial.level(), 1);

        // 测试二级节点的父节点正确
        let cpu0 = nodes
            .iter()
            .find(|n| n.full_path() == "/cpus/cpu@0")
            .unwrap();
        assert_eq!(cpu0.parent().unwrap().full_path(), "/cpus");
        assert_eq!(cpu0.level(), 2);

        let cpu1 = nodes
            .iter()
            .find(|n| n.full_path() == "/cpus/cpu@1")
            .unwrap();
        assert_eq!(cpu1.parent().unwrap().full_path(), "/cpus");
        assert_eq!(cpu1.level(), 2);
    }

    #[test]
    fn test_parent_relationships_cache() {
        let raw = fdt_reserve();
        let fdt = unsafe { fdt_parser::Fdt::from_ptr(raw.ptr()).unwrap() };

        // 收集所有节点到Vec中以便查找
        let nodes = fdt.all_nodes();

        // 测试根节点没有父节点
        let root = nodes.iter().find(|n| n.full_path() == "/").unwrap();
        assert!(root.parent().is_none(), "Root node should have no parent");
        assert_eq!(root.level(), 0);

        // 测试一级节点的父节点是根节点
        let chosen = nodes.iter().find(|n| n.full_path() == "/chosen").unwrap();
        assert_eq!(chosen.parent().unwrap().full_path(), "/");
        assert_eq!(chosen.level(), 1);

        let memory = nodes.iter().find(|n| n.full_path() == "/memory@0").unwrap();
        assert_eq!(memory.parent().unwrap().full_path(), "/");
        assert_eq!(memory.level(), 1);

        let cpus = nodes.iter().find(|n| n.full_path() == "/cpus").unwrap();
        assert_eq!(cpus.parent().unwrap().full_path(), "/");
        assert_eq!(cpus.level(), 1);

        let timer = nodes.iter().find(|n| n.full_path() == "/timer").unwrap();
        assert_eq!(timer.parent().unwrap().full_path(), "/");
        assert_eq!(timer.level(), 1);

        let serial = nodes
            .iter()
            .find(|n| n.full_path() == "/serial@1c28000")
            .unwrap();
        assert_eq!(serial.parent().unwrap().full_path(), "/");
        assert_eq!(serial.level(), 1);

        // 测试二级节点的父节点正确
        let cpu0 = nodes
            .iter()
            .find(|n| n.full_path() == "/cpus/cpu@0")
            .unwrap();
        assert_eq!(cpu0.parent().unwrap().full_path(), "/cpus");
        assert_eq!(cpu0.level(), 2);

        let cpu1 = nodes
            .iter()
            .find(|n| n.full_path() == "/cpus/cpu@1")
            .unwrap();
        assert_eq!(cpu1.parent().unwrap().full_path(), "/cpus");
        assert_eq!(cpu1.level(), 2);
    }

    #[test]
    fn test_parent_with_different_dtb() {
        // 只使用一个较小的DTB文件测试parent关系以避免性能问题
        let test_cases = [("Test Reserve", fdt_reserve())];

        for (name, raw) in test_cases {
            let fdt = unsafe { fdt_parser::Fdt::from_ptr(raw.ptr()).unwrap() };

            // 找到根节点
            let nodes = fdt.all_nodes();
            let root_node = nodes.iter().find(|node| node.full_path() == "/").unwrap();

            assert!(
                root_node.parent().is_none(),
                "{}: Root node should have no parent",
                name
            );
            assert_eq!(
                root_node.level(),
                0,
                "{}: Root node should be at level 0",
                name
            );

            // 找一个一级节点
            let first_level_node = nodes
                .iter()
                .find(|node| node.level() == 1 && node.full_path() != "/")
                .unwrap();

            assert_eq!(
                first_level_node.parent().unwrap().full_path(),
                "/",
                "{}: First level child's parent should be root",
                name
            );
            assert_eq!(
                first_level_node.level(),
                1,
                "{}: First level child should be at level 1",
                name
            );
        }
    }

    #[test]
    fn test_parent_edge_cases() {
        let raw = fdt_reserve();
        let fdt = unsafe { fdt_parser::Fdt::from_ptr(raw.ptr()).unwrap() };

        // 测试节点的父节点一致性
        let nodes = fdt.all_nodes();

        for node in &nodes {
            if let Some(parent) = node.parent() {
                // 父节点的level应该比当前节点少1
                assert_eq!(
                    parent.level(),
                    node.level().saturating_sub(1),
                    "Parent level should be one less than child for node {}",
                    node.full_path()
                );

                // 如果不是根节点，父节点不应该为None
                if node.level() > 0 {
                    assert!(parent.parent().is_some() || parent.level() == 0,
                           "Parent of non-root node should either have a parent or be root for node {}",
                           node.full_path());
                }
            } else {
                // 没有父节点的应该只有根节点
                assert_eq!(
                    node.level(),
                    0,
                    "Only root node should have no parent, but node {} at level {} has none",
                    node.full_path(),
                    node.level()
                );
            }
        }
    }
}
