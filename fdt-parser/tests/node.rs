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
    fn test_find_node1() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let node = fdt.find_node("/soc/timer").unwrap();
        assert_eq!(node.name, "timer@7e003000");
    }
    #[test]
    fn test_find_node2() {
        let fdt = Fdt::from_bytes(TEST_FDT).unwrap();
        let node = fdt.find_node("/soc/serial@7e215040").unwrap();
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
        let node = fdt.find_node("serial0").unwrap();
        assert_eq!(node.name, "serial@7e215040");
    }
}
