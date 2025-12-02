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
    assert!(
        !fdt.root.properties.is_empty(),
        "root should have properties"
    );

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

#[test]
fn test_find_by_path() {
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 测试根节点
    let root = fdt.find_by_path("/");
    assert!(root.is_some());
    assert!(root.unwrap().name.is_empty());

    // 测试 soc 节点
    let soc = fdt.find_by_path("/soc");
    assert!(soc.is_some());
    assert_eq!(soc.unwrap().name, "soc");

    // 测试相对路径（不带前导 /）
    let soc2 = fdt.find_by_path("soc");
    assert!(soc2.is_some());
    assert_eq!(soc2.unwrap().name, "soc");

    // 测试不存在的路径
    let not_found = fdt.find_by_path("/not/exist");
    assert!(not_found.is_none());
}

#[test]
fn test_find_by_name() {
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 查找所有名为 "clocks" 的节点
    let clocks_nodes = fdt.find_by_name("clocks");
    // 可能有多个或没有，但至少应该返回空 Vec 而不是 panic
    println!("Found {} nodes named 'clocks'", clocks_nodes.len());

    // 查找不存在的名称
    let not_found = fdt.find_by_name("this-node-does-not-exist");
    assert!(not_found.is_empty());
}

#[test]
fn test_find_by_name_prefix() {
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 查找所有以 "gpio" 开头的节点
    let gpio_nodes = fdt.find_by_name_prefix("gpio");
    println!("Found {} nodes starting with 'gpio'", gpio_nodes.len());

    // 所有找到的节点名称都应该以 "gpio" 开头
    for node in &gpio_nodes {
        assert!(
            node.name.starts_with("gpio"),
            "Node '{}' should start with 'gpio'",
            node.name
        );
    }
}

#[test]
fn test_find_all_by_path() {
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 查找根节点
    let root_nodes = fdt.find_all_by_path("/");
    assert_eq!(root_nodes.len(), 1);
    assert!(root_nodes[0].name.is_empty());

    // 使用通配符查找
    // 查找所有一级子节点
    let first_level = fdt.find_all_by_path("/*");
    assert!(!first_level.is_empty(), "should have first level children");
    println!("Found {} first level nodes", first_level.len());

    // 通配符测试：查找 /soc 下的所有子节点（如果存在 soc）
    let soc_children = fdt.find_all_by_path("/soc/*");
    println!("Found {} children under /soc", soc_children.len());
}

#[test]
fn test_find_by_phandle() {
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 遍历所有节点找到一个有 phandle 的节点
    fn find_phandle_node(node: &Node) -> Option<(Phandle, String)> {
        if let Some(phandle) = node.phandle() {
            return Some((phandle, node.name.clone()));
        }
        for child in &node.children {
            if let Some(result) = find_phandle_node(child) {
                return Some(result);
            }
        }
        None
    }

    // 如果找到了有 phandle 的节点，测试 find_by_phandle
    if let Some((phandle, name)) = find_phandle_node(&fdt.root) {
        let found = fdt.find_by_phandle(phandle);
        assert!(found.is_some(), "should find node by phandle");
        assert_eq!(found.unwrap().name, name);
    }
}

#[test]
fn test_find_by_path_mut() {
    let raw = fdt_qemu();
    let mut fdt = Fdt::from_bytes(&raw).unwrap();

    // 通过路径修改节点
    if let Some(memory) = fdt.find_by_path_mut("/memory@40000000") {
        memory.add_property(Property::raw_string("test-prop", "test-value"));
    }

    // 验证修改
    let memory = fdt.find_by_path("/memory@40000000");
    assert!(memory.is_some());
    assert!(memory.unwrap().find_property("test-prop").is_some());
}

#[test]
fn test_find_by_alias() {
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 获取所有别名
    let aliases = fdt.aliases();
    println!("Found {} aliases", aliases.len());
    for (name, path) in &aliases {
        println!("  {} -> {}", name, path);
    }

    // 如果有别名，测试通过别名查找
    if let Some((alias_name, expected_path)) = aliases.first() {
        // 通过别名查找
        let node = fdt.find_by_path(alias_name);
        assert!(node.is_some(), "should find node by alias '{}'", alias_name);

        // 通过完整路径查找
        let node_by_path = fdt.find_by_path(expected_path);
        assert!(
            node_by_path.is_some(),
            "should find node by path '{}'",
            expected_path
        );

        // 两种方式找到的应该是同一个节点
        assert_eq!(node.unwrap().name, node_by_path.unwrap().name);
    }
}
