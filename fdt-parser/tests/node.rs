#[cfg(test)]
mod test {
    use dtb_file::{fdt_3568, fdt_phytium, fdt_qemu, fdt_reserve, fdt_rpi_4b};
    use fdt_parser::*;

    #[test]
    fn test_new() {
        let raw = fdt_qemu();
        let ptr = raw.as_ptr() as *mut u8;
        let fdt = unsafe { Fdt::from_ptr(ptr).unwrap() };

        println!("ver: {:#?}", fdt.header().version);
    }

    #[test]
    fn test_memory_reservation_blocks() {
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
        let node = test_node();
        if let Some(node) = node {
            println!("{:?}", node.name());
        }
    }

    #[test]
    fn test_all_nodes() {
        env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .init();
        let raw = fdt_reserve();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };
        for node in fdt.all_nodes() {
            println!(
                "{}{} l{} parent={:?}",
                match node.level() {
                    0 => "",
                    1 => "  ",
                    2 => "    ",
                    _ => "       ",
                },
                node.full_path(),
                node.level(),
                node.parent_name()
            );
        }
    }

    #[test]
    fn test_property() {
        let raw = fdt_rpi_4b();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };
        for node in fdt.all_nodes() {
            println!("{}:", node.name());
            for prop in node.properties() {
                println!("  {:?}", prop);
            }
        }
    }

    #[test]
    fn test_str_list() {
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
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node = fdt.find_nodes("/soc/serial@7e215040")[0].clone();
        assert_eq!(node.name(), "serial@7e215040");
    }

    #[test]
    fn test_find_aliases() {
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let path = fdt.find_aliase("serial0").unwrap();
        assert_eq!(path, "/soc/serial@7e215040");
    }
    #[test]
    fn test_find_node_aliases() {
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node = fdt.find_nodes("serial0")[0].clone();
        assert_eq!(node.name(), "serial@7e215040");
    }

    // #[test]
    // fn test_reg() {
    //     let raw = fdt_rpi_4b();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();

    //     let node = fdt
    //         .find_nodes("/soc/serial@7e215040")
    //         .next()
    //         .unwrap()
    //         .unwrap();

    //     let reg = node.reg().unwrap().unwrap().next().unwrap();

    //     println!("reg: {:?}", reg);

    //     assert_eq!(
    //         reg.address, 0xfe215040,
    //         "want 0xfe215040, got {:#x}",
    //         reg.address
    //     );
    //     assert_eq!(
    //         reg.child_bus_address, 0x7e215040,
    //         "want 0x7e215040, got {:#x}",
    //         reg.child_bus_address
    //     );
    //     assert_eq!(
    //         reg.size,
    //         Some(0x40),
    //         "want 0x40, got {:#x}",
    //         reg.size.unwrap()
    //     );
    // }

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
        let raw = fdt_rpi_4b();
        let fdt = unsafe { Fdt::from_ptr(raw.ptr()).unwrap() };

        let ls = fdt.find_compatible(&["arm,pl011", "arm,primecell"]);

        assert_eq!(ls[0].name(), "serial@7e201000");
    }

    #[test]
    fn test_compatibles() {
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

    // #[test]
    // fn test_all_compatibles() {
    //     let raw = fdt_qemu();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();

    //     for node in fdt.all_nodes() {
    //         let node = node.unwrap();
    //         println!("{}", node.name());
    //         for cam in node.compatibles_flatten() {
    //             println!("   {}", cam);
    //         }
    //     }
    // }

    // #[test]
    // fn test_interrupt() {
    //     let raw = fdt_rpi_4b();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();
    //     let node = fdt
    //         .find_nodes("/soc/serial@7e215040")
    //         .next()
    //         .unwrap()
    //         .unwrap();

    //     let itr_ctrl = node.interrupt_parent().unwrap().unwrap();
    //     println!("itr_ctrl: {:?}", itr_ctrl.name());
    //     let interrupt_cells = itr_ctrl.interrupt_cells().unwrap();
    //     assert_eq!(interrupt_cells, 3);
    // }

    // #[test]
    // fn test_interrupt2() {
    //     let raw = fdt_rpi_4b();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();

    //     let node = fdt
    //         .find_compatible(&["brcm,bcm2711-hdmi0"])
    //         .next()
    //         .unwrap()
    //         .unwrap();

    //     let itr_ctrl_ph = node.get_interrupt_parent_phandle().unwrap();
    //     assert_eq!(itr_ctrl_ph, 0x2c.into());

    //     let itr_ctrl = node.interrupt_parent().unwrap().unwrap();
    //     assert_eq!(itr_ctrl.name(), "interrupt-controller@7ef00100");
    // }

    // #[test]
    // fn test_interrupts() {
    //     let raw = fdt_rpi_4b();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();

    //     let node = fdt
    //         .find_compatible(&["brcm,bcm2711-hdmi0"])
    //         .next()
    //         .unwrap()
    //         .unwrap();
    //     let itr = node.interrupts().unwrap().unwrap();
    //     let want_itrs = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5];

    //     for (i, o) in itr.enumerate() {
    //         let itr = o.collect::<Vec<_>>();
    //         assert_eq!(itr[0], want_itrs[i]);
    //     }
    // }

    // #[test]
    // fn test_clocks() {
    //     let raw = fdt_rpi_4b();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();

    //     let node = fdt
    //         .find_nodes("/soc/serial@7e215040")
    //         .next()
    //         .unwrap()
    //         .unwrap();
    //     let clocks = node
    //         .clocks()
    //         .unwrap()
    //         .collect::<Result<Vec<_>, _>>()
    //         .unwrap();
    //     for clock in &clocks {
    //         println!("clock: {:?}", clock);
    //     }
    //     let clock = &clocks[0];
    //     assert_eq!(clock.node.name(), "aux@7e215000");
    // }

    // #[test]
    // fn test_clocks_cell_1() {
    //     let fdt = Fdt::from_bytes(TEST_3568_FDT).unwrap();
    //     let node = fdt.find_nodes("/sdhci@fe310000").next().unwrap();
    //     let clocks = node.clocks().collect::<Vec<_>>();
    //     let clock = &clocks[0];

    //     for clock in &clocks {
    //         println!("clock: {:?}", clock);
    //     }
    //     assert_eq!(clock.node.name, "clock-controller@fdd20000");
    // }

    // #[test]
    // fn test_clocks_cell_0() {
    //     let fdt = Fdt::from_bytes(TEST_PHYTIUM_FDT).unwrap();
    //     let node = fdt.find_nodes("/soc/uart@2800e000").next().unwrap();
    //     let clocks = node.clocks().collect::<Vec<_>>();

    //     for clock in &clocks {
    //         println!("clock: {:?}", clock);
    //     }
    // }

    // #[test]
    // fn test_pcie() {
    //     let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
    //     let node = fdt.find_compatible(&["brcm,bcm2711-pcie"]).next().unwrap();
    //     let regs = node.reg().unwrap().collect::<Vec<_>>();
    //     let reg = regs[0];
    //     println!("reg: {:?}", reg);
    //     assert_eq!(reg.address, 0xfd500000);
    //     assert_eq!(reg.size, Some(0x9310));
    // }

    // #[test]
    // fn test_pci2() {
    //     let fdt = Fdt::from_bytes(TEST_PHYTIUM_FDT).unwrap();
    //     let pci = fdt
    //         .find_compatible(&["pci-host-ecam-generic"])
    //         .next()
    //         .unwrap()
    //         .into_pci()
    //         .unwrap();

    //     let want = [
    //         PciRange {
    //             space: PciSpace::IO,
    //             bus_address: 0x0,
    //             cpu_address: 0x50000000,
    //             size: 0xf00000,
    //             prefetchable: false,
    //         },
    //         PciRange {
    //             space: PciSpace::Memory32,
    //             bus_address: 0x58000000,
    //             cpu_address: 0x58000000,
    //             size: 0x28000000,
    //             prefetchable: false,
    //         },
    //         PciRange {
    //             space: PciSpace::Memory64,
    //             bus_address: 0x1000000000,
    //             cpu_address: 0x1000000000,
    //             size: 0x1000000000,
    //             prefetchable: false,
    //         },
    //     ];

    //     for (i, range) in pci.ranges().unwrap().enumerate() {
    //         assert_eq!(range, want[i]);
    //     }
    // }

    // #[test]
    // fn test_pci_irq_map() {
    //     let fdt = Fdt::from_bytes(TEST_PHYTIUM_FDT).unwrap();
    //     let pci = fdt
    //         .find_compatible(&["pci-host-ecam-generic"])
    //         .next()
    //         .unwrap()
    //         .into_pci()
    //         .unwrap();

    //     let irq = pci.child_interrupts(0, 0, 0, 4).unwrap();

    //     for one in irq.irqs {
    //         println!("one: {:?}", one);
    //     }
    // }

    // #[test]
    // fn test_pci_irq_map2() {
    //     let fdt = Fdt::from_bytes(TEST_QEMU_FDT).unwrap();
    //     let pci = fdt
    //         .find_compatible(&["pci-host-ecam-generic"])
    //         .next()
    //         .unwrap()
    //         .into_pci()
    //         .unwrap();

    //     let irq = pci.child_interrupts(0, 2, 0, 1).unwrap();

    //     let want = [0, 5, 4];

    //     for (got, want) in irq.irqs.zip(want.iter()) {
    //         assert_eq!(got, *want);
    //     }
    // }

    // #[test]
    // fn test_debugcon() {
    //     let raw = fdt_qemu();
    //     let fdt = Fdt::from_bytes(&raw).unwrap();
    //     let node = fdt.chosen().unwrap().unwrap().debugcon().unwrap().unwrap();
    //     println!("{:?}", node.name());
    // }

    // #[test]
    // fn test_debugcon2() {
    //     let fdt = Fdt::from_bytes(TEST_3568_FDT).unwrap();
    //     let node = fdt.chosen().unwrap().debugcon().unwrap();
    //     println!("{:?}", node.name);
    // }
}
