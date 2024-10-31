#[cfg(test)]
mod test {
    use fdt_parser::*;

    const TEST_FDT: &[u8] = include_bytes!("../../dtb/bcm2711-rpi-4-b.dtb");

    #[test]
    fn test_find_compatible() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let pl011 = fdt.find_compatible(&["arm,pl011"]).unwrap();
        assert_eq!(pl011.name, "serial@7e201000");
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

        let node = fdt.find_compatible(&["brcm,bcm2711-hdmi0"]).unwrap();
        let itr_ctrl = node.interrupt_parent().unwrap();

        assert_eq!(itr_ctrl.node.name, "interrupt-controller@7ef00100");
    }

    #[test]
    fn test_interrupts() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();

        let node = fdt.find_compatible(&["brcm,bcm2711-hdmi0"]).unwrap();
        let itr = node.interrupts().unwrap();
        assert_eq!(itr.cell_size, 1);
        let want_itrs = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5];

        for (i, o) in itr.interrupts().enumerate() {
            assert_eq!(o, want_itrs[i]);
        }
    }
}
