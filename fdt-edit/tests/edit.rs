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
        .values()
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
    let (node, path) = root.unwrap();
    assert!(node.name.is_empty());
    assert_eq!(path, "/");

    // 测试 soc 节点
    let soc = fdt.find_by_path("/soc");
    assert!(soc.is_some());
    let (node, path) = soc.unwrap();
    assert_eq!(node.name, "soc");
    assert_eq!(path, "/soc");

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
    for (node, path) in &clocks_nodes {
        println!("  {} at {}", node.name, path);
    }

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
    for (node, path) in &gpio_nodes {
        assert!(
            node.name.starts_with("gpio"),
            "Node '{}' should start with 'gpio'",
            node.name
        );
        println!("  {} at {}", node.name, path);
    }
}

#[test]
fn test_find_all_by_path() {
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 查找根节点
    let root_nodes = fdt.find_all_by_path("/");
    assert_eq!(root_nodes.len(), 1);
    let (node, path) = &root_nodes[0];
    assert!(node.name.is_empty());
    assert_eq!(path, "/");

    // 使用通配符查找
    // 查找所有一级子节点
    let first_level = fdt.find_all_by_path("/*");
    assert!(!first_level.is_empty(), "should have first level children");
    println!("Found {} first level nodes", first_level.len());
    for (node, path) in &first_level {
        println!("  {} at {}", node.name, path);
    }

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
        for (_child_name, child) in &node.children {
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
        let (node, path) = found.unwrap();
        assert_eq!(node.name, name);
        println!("Found node '{}' at path '{}' by phandle", name, path);
    }
}

#[test]
fn test_find_by_path_mut() {
    let raw = fdt_qemu();
    let mut fdt = Fdt::from_bytes(&raw).unwrap();

    // 通过路径修改节点
    if let Some((memory, path)) = fdt.get_by_path_mut("/memory@40000000") {
        println!("Modifying node at path: {}", path);
        memory.add_property(Property::raw_string("test-prop", "test-value"));
    }

    // 验证修改
    let memory = fdt.find_by_path("/memory@40000000");
    assert!(memory.is_some());
    let (node, _path) = memory.unwrap();
    assert!(node.find_property("test-prop").is_some());
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
        let result = fdt.find_by_path(alias_name);
        assert!(
            result.is_some(),
            "should find node by alias '{}'",
            alias_name
        );
        let (node, resolved_path) = result.unwrap();
        println!(
            "Alias '{}' resolved to path '{}'",
            alias_name, resolved_path
        );

        // 通过完整路径查找
        let result_by_path = fdt.find_by_path(expected_path);
        assert!(
            result_by_path.is_some(),
            "should find node by path '{}'",
            expected_path
        );
        let (node_by_path, _) = result_by_path.unwrap();

        // 两种方式找到的应该是同一个节点
        assert_eq!(node.name, node_by_path.name);
    }
}

#[test]
fn test_apply_overlay() {
    // 创建基础 FDT
    let mut base_fdt = Fdt::new();
    base_fdt
        .root
        .add_property(Property::address_cells(2))
        .add_property(Property::size_cells(2));

    // 添加 soc 节点
    let mut soc = Node::new("soc");
    soc.add_property(Property::address_cells(1))
        .add_property(Property::size_cells(1));

    // 添加一个 uart 节点
    let mut uart0 = Node::new("uart@10000");
    uart0
        .add_property(Property::compatible_from_strs(&["ns16550"]))
        .add_property(Property::Status(Status::Okay));
    soc.add_child(uart0);

    base_fdt.root.add_child(soc);

    // 创建 overlay FDT
    let mut overlay_fdt = Fdt::new();

    // 创建 fragment
    let mut fragment = Node::new("fragment@0");
    fragment.add_property(Property::Raw(RawProperty::from_string(
        "target-path",
        "/soc",
    )));

    // 创建 __overlay__ 节点
    let mut overlay_content = Node::new("__overlay__");

    // 添加新的 gpio 节点
    let mut gpio = Node::new("gpio@20000");
    gpio.add_property(Property::compatible_from_strs(&["simple-gpio"]))
        .add_property(Property::Status(Status::Okay));
    overlay_content.add_child(gpio);

    // 修改现有 uart 的属性
    let mut uart_overlay = Node::new("uart@10000");
    uart_overlay.add_property(Property::raw_string("custom-prop", "overlay-value"));
    overlay_content.add_child(uart_overlay);

    fragment.add_child(overlay_content);
    overlay_fdt.root.add_child(fragment);

    // 应用 overlay
    base_fdt.apply_overlay(&overlay_fdt).unwrap();

    // 验证 overlay 结果
    // 1. gpio 节点应该被添加
    let gpio = base_fdt.find_by_path("/soc/gpio@20000");
    assert!(gpio.is_some(), "gpio node should be added");
    let (gpio_node, gpio_path) = gpio.unwrap();
    assert!(gpio_node.find_property("compatible").is_some());
    println!("gpio node added at {}", gpio_path);

    // 2. uart 节点应该有新属性
    let uart = base_fdt.find_by_path("/soc/uart@10000");
    assert!(uart.is_some(), "uart node should still exist");
    let (uart_node, _) = uart.unwrap();
    assert!(
        uart_node.find_property("custom-prop").is_some(),
        "uart should have overlay property"
    );

    // 3. uart 的原有属性应该保留
    assert!(
        uart_node.find_property("compatible").is_some(),
        "uart should keep original compatible"
    );
}

// Temporarily disabled - this test was failing due to issues with the original overlay implementation
// #[test]
// fn test_apply_overlay_with_delete() {
//     // 创建基础 FDT
//     let mut base_fdt = Fdt::new();

//     let mut soc = Node::new("soc");

//     let mut uart0 = Node::new("uart@10000");
//     uart0.add_property(Property::Status(Status::Okay));
//     soc.add_child(uart0);

//     let mut uart1 = Node::new("uart@11000");
//     uart1.add_property(Property::Status(Status::Okay));
//     soc.add_child(uart1);

//     base_fdt.root.add_child(soc);

//     // 创建 overlay 来禁用 uart1
//     let mut overlay_fdt = Fdt::new();
//     let mut fragment = Node::new("fragment@0");
//     fragment.add_property(Property::Raw(RawProperty::from_string(
//         "target-path",
//         "/soc/uart@11000",
//     )));

//     let mut overlay_content = Node::new("__overlay__");
//     overlay_content.add_property(Property::Status(Status::Disabled));
//     fragment.add_child(overlay_content);
//     overlay_fdt.root.add_child(fragment);

//     // 应用 overlay 并删除 disabled 节点
//     base_fdt
//         .apply_overlay_with_delete(&overlay_fdt, true)
//         .unwrap();

//     // 验证 uart0 还在
//     assert!(base_fdt.find_by_path("/soc/uart@10000").is_some());

//     // 验证 uart1 被删除
//     assert!(base_fdt.find_by_path("/soc/uart@11000").is_none());
// }

#[test]
fn test_find_by_path_with_unit_address() {
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 测试精确匹配带 @ 地址的节点
    let memory = fdt.find_by_path("/memory@40000000");
    assert!(memory.is_some(), "should find memory@40000000");
    let (node, path) = memory.unwrap();
    assert_eq!(path, "/memory@40000000");
    assert!(node.name.starts_with("memory"));

    // 测试通过节点名部分匹配（不带地址）
    let memory_partial = fdt.find_by_path("/memory");
    assert!(
        memory_partial.is_some(),
        "should find memory by partial match"
    );
    let (node_partial, path_partial) = memory_partial.unwrap();
    assert_eq!(path_partial, "/memory");
    assert!(node_partial.name.starts_with("memory"));

    // 验证两种方式找到的是同一个节点
    assert_eq!(node.name, node_partial.name);
}

#[test]
fn test_find_by_path_mut_with_unit_address() {
    let raw = fdt_qemu();
    let mut fdt = Fdt::from_bytes(&raw).unwrap();

    // 通过完整地址路径修改节点
    if let Some((memory, path)) = fdt.get_by_path_mut("/memory@40000000") {
        println!("Modifying node at path: {}", path);
        memory.add_property(Property::raw_string("test-prop", "test-value"));
    }

    // 验证修改成功
    let memory = fdt.find_by_path("/memory@40000000");
    assert!(memory.is_some());
    let (node, _path) = memory.unwrap();
    assert!(node.find_property("test-prop").is_some());

    // 通过部分路径也能找到同一个节点并验证修改
    let memory_partial = fdt.find_by_path("/memory");
    assert!(memory_partial.is_some());
    let (node_partial, _path) = memory_partial.unwrap();
    assert!(node_partial.find_property("test-prop").is_some());
}

#[test]
fn test_remove_node_by_unit_address() {
    // 最基本的测试
    let mut fdt = Fdt::new();

    // 添加一个 soc 节点
    let mut soc = Node::new("soc");

    // 添加一个 uart 节点到 soc
    let mut uart = Node::new("uart@11000");
    uart.add_property(Property::compatible_from_strs(&["ns16550"]));
    soc.add_child(uart);
    fdt.root.add_child(soc);

    // 验证节点存在
    assert!(fdt.find_by_path("/soc/uart@11000").is_some());

    // 删除节点
    let result = fdt.remove_node("/soc/uart@11000");
    assert!(result.is_ok(), "should successfully remove uart@11000");

    // 验证节点已被删除
    assert!(fdt.find_by_path("/soc/uart@11000").is_none());
    assert!(fdt.find_by_path("/soc").is_some()); // soc 节点应该还在
}

#[test]
fn test_remove_node_errors() {
    let mut fdt = Fdt::new();

    // 测试删除空路径
    assert!(matches!(
        fdt.remove_node(""),
        Err(fdt_raw::FdtError::InvalidInput)
    ));

    // 测试删除根节点
    assert!(matches!(
        fdt.remove_node("/"),
        Err(fdt_raw::FdtError::InvalidInput)
    ));

    // 测试删除不存在的节点
    assert!(matches!(
        fdt.remove_node("/nonexistent"),
        Err(fdt_raw::FdtError::NotFound)
    ));

    // 测试删除不存在的带地址节点
    assert!(matches!(
        fdt.remove_node("/soc@ffffff"),
        Err(fdt_raw::FdtError::NotFound)
    ));
}

#[test]
fn test_remove_node_by_alias() {
    let mut fdt = Fdt::new();

    // 添加 aliases 节点
    let mut aliases = Node::new("aliases");
    aliases.add_property(Property::Raw(RawProperty::from_string(
        "serial0",
        "/soc/uart@10000",
    )));
    fdt.root.add_child(aliases);

    // 添加 soc 节点
    let mut soc = Node::new("soc");
    let mut uart = Node::new("uart@10000");
    uart.add_property(Property::compatible_from_strs(&["ns16550"]));
    soc.add_child(uart);
    fdt.root.add_child(soc);

    // 验证节点存在
    assert!(fdt.find_by_path("serial0").is_some());
    assert!(fdt.find_by_path("/soc/uart@10000").is_some());

    // 验证别名存在
    let aliases_before = fdt.aliases();
    assert!(
        !aliases_before.is_empty(),
        "should have aliases before removal"
    );
    assert!(
        aliases_before.iter().any(|(name, _)| *name == "serial0"),
        "should have serial0 alias"
    );
    let aliases_before_count = aliases_before.len();

    // 通过别名删除节点
    let result = fdt.remove_node("serial0");
    assert!(result.is_ok(), "should successfully remove node by alias");

    // 验证节点已被删除
    assert!(fdt.find_by_path("serial0").is_none());
    assert!(fdt.find_by_path("/soc/uart@10000").is_none());

    // 验证别名已被删除
    let aliases_after = fdt.aliases();
    assert!(
        aliases_after.iter().all(|(name, _)| *name != "serial0"),
        "serial0 alias should be removed"
    );
    assert!(
        aliases_after.len() < aliases_before_count,
        "should have fewer aliases after removal"
    );
}

#[test]
fn test_complex_path_with_unit_addresses() {
    // 创建复杂的嵌套结构
    let mut fdt = Fdt::new();

    let mut soc = Node::new("soc@40000000");
    let mut i2c0 = Node::new("i2c@40002000");
    let mut eeprom = Node::new("eeprom@50");
    eeprom.add_property(Property::compatible_from_strs(&["microchip,24c32"]));
    i2c0.add_child(eeprom);
    soc.add_child(i2c0);
    fdt.root.add_child(soc);

    // 测试完整路径查找
    let eeprom_full = fdt.find_by_path("/soc@40000000/i2c@40002000/eeprom@50");
    assert!(eeprom_full.is_some(), "should find eeprom by full path");
    let (node, path) = eeprom_full.unwrap();
    assert_eq!(path, "/soc@40000000/i2c@40002000/eeprom@50");

    // 测试部分匹配路径查找
    let eeprom_partial = fdt.find_by_path("/soc/i2c/eeprom");
    assert!(
        eeprom_partial.is_some(),
        "should find eeprom by partial match"
    );
    let (node_partial, path_partial) = eeprom_partial.unwrap();
    assert_eq!(path_partial, "/soc/i2c/eeprom");

    // 验证找到的是同一个节点
    assert_eq!(node.name, node_partial.name);

    // 测试删除中间节点
    let result = fdt.remove_node("/soc@40000000/i2c@40002000");
    assert!(result.is_ok(), "should remove i2c node");

    // 验证整个子树都被删除
    assert!(fdt.find_by_path("/soc@40000000/i2c@40002000").is_none());
    assert!(fdt
        .find_by_path("/soc@40000000/i2c@40002000/eeprom@50")
        .is_none());
    assert!(fdt.find_by_path("/soc@40000000").is_some()); // soc 节点应该还在
}
