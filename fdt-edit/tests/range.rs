use dtb_file::*;
use fdt_edit::*;

#[test]
fn test_reg_address_translation() {
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 测试 /soc/serial@7e215040 节点
    // bus address: 0x7e215040, CPU address: 0xfe215040
    let node = fdt.get_by_path("/soc/serial@7e215040").unwrap();
    let regs = node.regs();

    assert!(!regs.is_empty(), "should have at least one reg entry");

    let reg = &regs[0];
    assert_eq!(reg.address, 0xfe215040, "CPU address should be 0xfe215040");
    assert_eq!(reg.child_bus_address, 0x7e215040, "bus address should be 0x7e215040");
    assert_eq!(reg.size, Some(0x40), "size should be 0x40");
}
