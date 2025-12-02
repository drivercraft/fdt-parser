#![cfg(not(target_os = "none"))]

use dtb_file::*;
use fdt_edit::*;

#[test]
fn test_parse_and_rebuild() {
    // 解析原始 DTB
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 验证根节点
    assert!(fdt.root.name.is_empty(), "root node should have empty name");

    // 验证有属性
    assert!(!fdt.root.properties.is_empty(), "root should have properties");

    // 验证有子节点
    assert!(!fdt.root.children.is_empty(), "root should have children");

    // 查找 memory 节点
    let has_memory = fdt
        .root
        .children
        .iter()
        .any(|c| c.name.starts_with("memory"));
    assert!(has_memory, "should have memory node");

    // 重新序列化
    let rebuilt = fdt.to_bytes();

    // 验证重建后的数据可以被重新解析
    let reparsed = Fdt::from_bytes(&rebuilt).unwrap();

    // 验证基本结构一致
    assert_eq!(fdt.root.children.len(), reparsed.root.children.len());
    assert_eq!(fdt.root.properties.len(), reparsed.root.properties.len());
}

#[test]
fn test_create_fdt() {
    let mut fdt = Fdt::new();

    // 设置根节点属性
    fdt.root
        .add_property(Property::address_cells(2))
        .add_property(Property::size_cells(2))
        .add_property(Property::compatible_from_strs(&["linux,dummy-virt"]));

    // 添加 memory 节点
    let mut memory = Node::new("memory@80000000");
    memory
        .add_property(Property::device_type("memory"))
        .add_property(Property::reg(
            vec![RegEntry::with_size(0x80000000, 0x40000000)], // 1GB at 0x80000000
            2,
            2,
        ));
    fdt.root.add_child(memory);

    // 添加 chosen 节点
    let mut chosen = Node::new("chosen");
    chosen.add_property(Property::raw_string(
        "bootargs",
        "console=ttyS0 earlycon=sbi",
    ));
    fdt.root.add_child(chosen);

    // 序列化
    let data = fdt.to_bytes();

    // 验证可以被解析
    let reparsed = Fdt::from_bytes(&data).unwrap();
    assert_eq!(reparsed.root.children.len(), 2);

    // 验证 memory 节点
    let memory = reparsed.root.find_child("memory@80000000").unwrap();
    assert!(memory.find_property("reg").is_some());
    assert!(memory.find_property("device_type").is_some());
}

#[test]
fn test_modify_fdt() {
    // 解析现有 DTB
    let raw = fdt_qemu();
    let mut fdt = Fdt::from_bytes(&raw).unwrap();

    // 修改 compatible 属性
    fdt.root
        .set_property(Property::compatible_from_strs(&["my-custom-board"]));

    // 添加新节点
    let mut new_node = Node::new("my-device@1000");
    new_node
        .add_property(Property::compatible_from_strs(&["vendor,my-device"]))
        .add_property(Property::reg(
            vec![RegEntry::with_size(0x1000, 0x100)],
            2,
            2,
        ))
        .add_property(Property::status_okay());
    fdt.root.add_child(new_node);

    // 序列化并重新解析
    let data = fdt.to_bytes();
    let reparsed = Fdt::from_bytes(&data).unwrap();

    // 验证修改
    let my_device = reparsed.root.find_child("my-device@1000").unwrap();
    assert!(my_device.find_property("compatible").is_some());
}

#[test]
fn test_memory_reservations() {
    let mut fdt = Fdt::new();

    // 添加内存保留
    fdt.memory_reservations.push(MemoryReservation {
        address: 0x40000000,
        size: 0x1000,
    });

    fdt.root.add_property(Property::address_cells(2));
    fdt.root.add_property(Property::size_cells(2));

    // 序列化并重新解析
    let data = fdt.to_bytes();
    let reparsed = Fdt::from_bytes(&data).unwrap();

    // 验证内存保留
    assert_eq!(reparsed.memory_reservations.len(), 1);
    assert_eq!(reparsed.memory_reservations[0].address, 0x40000000);
    assert_eq!(reparsed.memory_reservations[0].size, 0x1000);
}

#[test]
fn test_rpi4b_parse() {
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 验证基本结构
    assert!(!fdt.root.children.is_empty());

    // 查找 soc 节点
    let soc = fdt.root.find_child("soc");
    assert!(soc.is_some(), "should have soc node");

    if let Some(soc) = soc {
        // soc 节点应该有很多子节点
        assert!(!soc.children.is_empty());
    }

    // 重建并验证
    let rebuilt = fdt.to_bytes();
    let reparsed = Fdt::from_bytes(&rebuilt).unwrap();
    assert_eq!(fdt.root.children.len(), reparsed.root.children.len());
}
