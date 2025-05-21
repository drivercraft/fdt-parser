#[cfg(test)]
mod test {
    use fdt_parser::*;

    const TEST_FDT: &[u8] = include_bytes!("../../dtb/bcm2711-rpi-4-b.dtb");
    const TEST_PHYTIUM_FDT: &[u8] = include_bytes!("../../dtb/phytium.dtb");
    const TEST_QEMU_FDT: &[u8] = include_bytes!("../../dtb/qemu_pci.dtb");
    const TEST_3568_FDT: &[u8] = include_bytes!("../../dtb/rk3568-firefly-roc-pc-se.dtb");

    #[test]
    fn test_str_list() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let uart = fdt.find_nodes("/soc/serial@7e201000").next().unwrap();
        let caps = uart
            .find_property("compatible")
            .unwrap()
            .str_list()
            .collect::<Vec<_>>();

        let want = ["arm,pl011", "arm,primecell"];

        for (i, cap) in caps.iter().enumerate() {
            assert_eq!(*cap, want[i]);
        }
    }

    #[test]
    fn test_find_compatible() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let pl011 = fdt
            .find_compatible(&["arm,pl011", "arm,primecell"])
            .next()
            .unwrap();
        assert_eq!(pl011.name, "serial@7e201000");
    }

    #[test]
    fn test_compatibles() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let uart = fdt.find_nodes("/soc/serial@7e201000").next().unwrap();
        let caps = uart.compatibles().collect::<Vec<_>>();

        let want = ["arm,pl011", "arm,primecell"];

        for (i, cap) in caps.iter().enumerate() {
            assert_eq!(*cap, want[i]);
        }
    }

    #[test]
    fn test_all_compatibles() {
        let fdt = Fdt::from_bytes(TEST_QEMU_FDT).unwrap();
        for node in fdt.all_nodes() {
            println!("{}", node.name);
            for cam in node.compatibles() {
                println!("   {}", cam);
            }
        }
    }

    #[test]
    fn test_find_nodes() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let uart = fdt.find_nodes("/soc/serial");
        let want = [
            "serial@7e201000",
            "serial@7e215040",
            "serial@7e201400",
            "serial@7e201600",
            "serial@7e201800",
            "serial@7e201a00",
        ];

        for (i, timer) in uart.enumerate() {
            assert_eq!(timer.name, want[i]);
        }
    }

    #[test]
    fn test_find_node2() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let node = fdt.find_nodes("/soc/serial@7e215040").next().unwrap();
        assert_eq!(node.name, "serial@7e215040");
    }
    #[test]
    fn test_find_aliases() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let path = fdt.find_aliase("serial0").unwrap();
        assert_eq!(path, "/soc/serial@7e215040");
    }
    #[test]
    fn test_find_node_aliases() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let node = fdt.find_nodes("serial0").next().unwrap();
        assert_eq!(node.name, "serial@7e215040");
    }

    #[test]
    fn test_chosen() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let chosen = fdt.chosen().unwrap();
        let bootargs = chosen.bootargs().unwrap();
        assert_eq!(
            bootargs,
            "coherent_pool=1M 8250.nr_uarts=1 snd_bcm2835.enable_headphones=0"
        );

        let stdout = chosen.stdout().unwrap();
        assert_eq!(stdout.params, Some("115200n8"));
        assert_eq!(stdout.node.name, "serial@7e215040");
    }

    #[test]
    fn test_reg() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let node = fdt.find_nodes("/soc/serial@7e215040").next().unwrap();

        let reg = node.reg().unwrap().next().unwrap();

        assert_eq!(reg.address, 0xfe215040);
        assert_eq!(reg.child_bus_address, 0x7e215040);
        assert_eq!(reg.size, Some(0x40));
    }

    #[test]
    fn test_interrupt() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let node = fdt.find_nodes("/soc/serial@7e215040").next().unwrap();

        let itr_ctrl = node.interrupt_parent().unwrap();

        assert_eq!(itr_ctrl.interrupt_cells(), 3);
    }

    #[test]
    fn test_interrupt2() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();

        let node = fdt.find_compatible(&["brcm,bcm2711-hdmi0"]).next().unwrap();
        let itr_ctrl = node.interrupt_parent().unwrap();

        assert_eq!(itr_ctrl.node.name, "interrupt-controller@7ef00100");
    }

    #[test]
    fn test_interrupts() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();

        let node = fdt.find_compatible(&["brcm,bcm2711-hdmi0"]).next().unwrap();
        let itr = node.interrupts().unwrap();
        let want_itrs = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5];

        for (i, o) in itr.enumerate() {
            let itr = o.collect::<Vec<_>>();
            assert_eq!(itr[0], want_itrs[i]);
        }
    }

    #[test]
    fn test_clocks() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let node = fdt.find_nodes("/soc/serial@7e215040").next().unwrap();
        let clocks = node.clocks().collect::<Vec<_>>();
        let clock = &clocks[0];
        for clock in &clocks {
            println!("clock: {:?}", clock);
        }
        assert_eq!(clock.node.name, "aux@7e215000");
    }

    #[test]
    fn test_clocks_cell_1() {
        let fdt = Fdt::from_bytes(TEST_3568_FDT).unwrap();
        let node = fdt.find_nodes("/sdhci@fe310000").next().unwrap();
        let clocks = node.clocks().collect::<Vec<_>>();
        let clock = &clocks[0];

        for clock in &clocks {
            println!("clock: {:?}", clock);
        }
        assert_eq!(clock.node.name, "clock-controller@fdd20000");
    }

    #[test]
    fn test_clocks_cell_0() {
        let fdt = Fdt::from_bytes(TEST_PHYTIUM_FDT).unwrap();
        let node = fdt.find_nodes("/soc/uart@2800e000").next().unwrap();
        let clocks = node.clocks().collect::<Vec<_>>();

        for clock in &clocks {
            println!("clock: {:?}", clock);
        }
    }

    #[test]
    fn test_pcie() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let node = fdt.find_compatible(&["brcm,bcm2711-pcie"]).next().unwrap();
        let regs = node.reg().unwrap().collect::<Vec<_>>();
        let reg = regs[0];
        println!("reg: {:?}", reg);
        assert_eq!(reg.address, 0xfd500000);
        assert_eq!(reg.size, Some(0x9310));
    }

    #[test]
    fn test_pci2() {
        let fdt = Fdt::from_bytes(TEST_PHYTIUM_FDT).unwrap();
        let pci = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .next()
            .unwrap()
            .into_pci()
            .unwrap();

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

        for (i, range) in pci.ranges().unwrap().enumerate() {
            assert_eq!(range, want[i]);
        }
    }

    #[test]
    fn test_pci_irq_map() {
        let fdt = Fdt::from_bytes(TEST_PHYTIUM_FDT).unwrap();
        let pci = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .next()
            .unwrap()
            .into_pci()
            .unwrap();

        let irq = pci.child_interrupts(0, 0, 0, 4).unwrap();

        for one in irq.irqs {
            println!("one: {:?}", one);
        }
    }

    #[test]
    fn test_pci_irq_map2() {
        let fdt = Fdt::from_bytes(TEST_QEMU_FDT).unwrap();
        let pci = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .next()
            .unwrap()
            .into_pci()
            .unwrap();

        let irq = pci.child_interrupts(0, 2, 0, 1).unwrap();

        let want = [0, 5, 4];

        for (got, want) in irq.irqs.zip(want.iter()) {
            assert_eq!(got, *want);
        }
    }

    #[test]
    fn test_debugcon() {
        let fdt = Fdt::from_bytes(TEST_QEMU_FDT).unwrap();
        let node = fdt.chosen().unwrap().debugcon().unwrap();
        println!("{:?}", node.name);
    }

    #[test]
    fn test_debugcon2() {
        let fdt = Fdt::from_bytes(TEST_3568_FDT).unwrap();
        let node = fdt.chosen().unwrap().debugcon().unwrap();
        println!("{:?}", node.name);
    }
}
