#[cfg(test)]
mod test {
    use dtb_file::{fdt_3568, fdt_phytium, fdt_qemu, fdt_reserve, fdt_rpi_4b};
    use fdt_parser::*;

    #[test]
    fn test_new() {
        let raw = fdt_qemu();
        let ptr = raw.as_ptr() as *mut u8;
        let fdt: Fdt<'static> = unsafe { Fdt::from_ptr(ptr).unwrap() };

        println!("ver: {:#?}", fdt.header().version);
    }

    #[test]
    fn test_memory_reservation_blocks() {
        // Test with custom DTB that has memory reservations
        let raw = fdt_reserve();
        let ptr = raw.as_ptr() as *mut u8;
        let fdt = unsafe { FdtNoMem::from_ptr(ptr).unwrap() };

        // Get memory reservation blocks
        let rsv_result = fdt.memory_reservaion_blocks();

        let entries: Vec<_> = rsv_result.collect();

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
        let rsv1: Vec<_> = fdt.memory_reservaion_blocks().collect();
        let rsv2: Vec<_> = fdt.memory_reservaion_blocks().collect();
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
            let fdt = unsafe { FdtNoMem::from_ptr(ptr).unwrap() };

            let rsv_result = fdt.memory_reservaion_blocks();

            let entries: Vec<_> = rsv_result.collect();
            assert_eq!(
                entries.len(),
                0,
                "{} DTB should have no memory reservation blocks",
                name
            );
        }
    }

    fn test_node<'a>() -> Option<Node<'a>> {
        let raw = fdt_rpi_4b();
        let fdt = unsafe { FdtNoMem::from_ptr(raw.ptr()).unwrap() };
        fdt.all_nodes().next().and_then(|n| n.ok())
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
        let fdt = unsafe { FdtNoMem::from_ptr(raw.ptr()).unwrap() };
        for node in fdt.all_nodes().flatten() {
            println!(
                "{}{} l{} parent={:?}",
                match node.level {
                    0 => "",
                    1 => "  ",
                    2 => "    ",
                    _ => "       ",
                },
                node.name(),
                node.level(),
                node.parent_name()
            );
        }
    }

    #[test]
    fn test_property() {
        let raw = fdt_rpi_4b();
        let fdt = unsafe { FdtNoMem::from_ptr(raw.ptr()).unwrap() };
        for node in fdt.all_nodes().flatten() {
            println!("{}:", node.name());
            for prop in node.properties().flatten() {
                println!("  {:?}", prop);
            }
        }
    }

    #[test]
    fn test_str_list() {
        let raw = fdt_rpi_4b();
        let fdt = unsafe { FdtNoMem::from_ptr(raw.ptr()).unwrap() };
        let uart = fdt
            .find_nodes("/soc/serial@7e201000")
            .next()
            .unwrap()
            .unwrap();
        let caps = uart
            .find_property("compatible")
            .unwrap()
            .unwrap()
            .str_list()
            .collect::<Vec<_>>();

        let want = ["arm,pl011", "arm,primecell"];

        for (i, cap) in caps.iter().enumerate() {
            assert_eq!(*cap, want[i]);
        }
    }
    #[test]
    fn test_find_nodes() {
        let raw = fdt_rpi_4b();
        let fdt = unsafe { FdtNoMem::from_ptr(raw.ptr()).unwrap() };

        let uart = fdt.find_nodes("/soc/serial");

        let want = [
            "serial@7e201000",
            "serial@7e215040",
            "serial@7e201400",
            "serial@7e201600",
            "serial@7e201800",
            "serial@7e201a00",
        ];

        for (act, want) in uart.zip(want.iter()) {
            let act = act.unwrap();
            assert_eq!(act.name(), *want);
        }
    }

    #[test]
    fn test_find_node2() {
        let raw = fdt_rpi_4b();
        let fdt = FdtNoMem::from_bytes(&raw).unwrap();
        let node = fdt
            .find_nodes("/soc/serial@7e215040")
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(node.name(), "serial@7e215040");
    }

    #[test]
    fn test_find_aliases() {
        let raw = fdt_rpi_4b();
        let fdt = FdtNoMem::from_bytes(&raw).unwrap();
        let path = fdt.find_aliase("serial0").unwrap();
        assert_eq!(path, "/soc/serial@7e215040");
    }
    #[test]
    fn test_find_node_aliases() {
        let raw = fdt_rpi_4b();
        let fdt = FdtNoMem::from_bytes(&raw).unwrap();
        let node = fdt.find_nodes("serial0").next().unwrap().unwrap();
        assert_eq!(node.name(), "serial@7e215040");
    }

    #[test]
    fn test_chosen() {
        let raw = fdt_rpi_4b();
        let fdt = FdtNoMem::from_bytes(&raw).unwrap();
        let chosen = fdt.chosen().unwrap().unwrap();
        let bootargs = chosen.bootargs().unwrap().unwrap();
        assert_eq!(
            bootargs,
            "coherent_pool=1M 8250.nr_uarts=1 snd_bcm2835.enable_headphones=0"
        );

        let stdout = chosen.stdout().unwrap().unwrap();
        assert_eq!(stdout.params, Some("115200n8"));
        assert_eq!(stdout.name(), "serial@7e215040");
    }

    #[test]
    fn test_reg() {
        let raw = fdt_rpi_4b();
        let fdt = FdtNoMem::from_bytes(&raw).unwrap();

        let node = fdt
            .find_nodes("/soc/serial@7e215040")
            .next()
            .unwrap()
            .unwrap();

        let reg = node.reg().unwrap().unwrap().next().unwrap();

        println!("reg: {:?}", reg);

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

    #[test]
    fn test_memory() {
        let raw = fdt_qemu();
        let fdt = FdtNoMem::from_bytes(&raw).unwrap();

        let node = fdt.memory();

        for node in node {
            let node = node.unwrap();
            println!("memory node: {:?}", node.name());
            for reg in node.reg().unwrap().unwrap() {
                println!("  reg: {:?}", reg);
            }

            for region in node.regions() {
                let region = region.unwrap();
                println!("  region: {:?}", region);
            }
        }
    }

    #[test]
    fn test_reserved_memory() {
        let raw = fdt_rpi_4b();
        let fdt = unsafe { FdtNoMem::from_ptr(raw.ptr()).unwrap() };
        let ls = fdt
            .reserved_memory_regions()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let want_names = ["linux,cma", "nvram@0", "nvram@1"];

        for node in &ls {
            println!("reserved memory node: {:?}", node);
        }

        assert_eq!(ls.len(), want_names.len());
        for (i, node) in ls.iter().enumerate() {
            assert_eq!(node.name(), want_names[i]);
        }
    }

    #[test]
    fn test_debugcon() {
        let raw = fdt_qemu();
        let fdt = FdtNoMem::from_bytes(&raw).unwrap();
        let node = fdt.chosen().unwrap().unwrap().debugcon().unwrap().unwrap();
        println!("{:?}", node.name());
    }

    #[test]
    fn test_debugcon2() {
        let raw = fdt_3568();
        let fdt = FdtNoMem::from_bytes(&raw).unwrap();
        let node = fdt.chosen().unwrap().unwrap().debugcon().unwrap().unwrap();
        println!("{:?}", node.name());
    }
}
