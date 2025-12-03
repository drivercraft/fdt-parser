#[cfg(test)]
mod tests {
    use fdt_edit::*;
    use fdt_edit::{Property, Status};

    #[test]
    fn test_remove_node_exact_path() {
        let mut root = Node::root();

        // 添加一些子节点
        let mut gpio0 = Node::new("gpio@1000");
        let mut gpio1 = Node::new("gpio@2000");
        let mut uart = Node::new("uart@3000");

        root.add_child(gpio0);
        root.add_child(gpio1);
        root.add_child(uart);

        // 测试精确删除
        let removed = root.remove_by_path("gpio@1000").unwrap();
        assert!(removed.is_some(), "Should remove gpio@1000");
        assert_eq!(removed.unwrap().name, "gpio@1000");

        // 验证节点已删除
        assert!(root.find_child_exact("gpio@1000").is_none());
        assert!(root.find_child_exact("gpio@2000").is_some());
        assert!(root.find_child_exact("uart@3000").is_some());
    }

    #[test]
    fn test_remove_node_nested_path() {
        let mut root = Node::root();

        // 创建嵌套结构
        let mut soc = Node::new("soc");
        let mut i2c0 = Node::new("i2c@0");
        let mut eeprom = Node::new("eeprom@50");

        i2c0.add_child(eeprom);
        soc.add_child(i2c0);
        root.add_child(soc);

        // 测试嵌套删除
        let removed = root.remove_by_path("soc/i2c@0/eeprom@50").unwrap();
        assert!(removed.is_some(), "Should remove eeprom@50");
        assert_eq!(removed.unwrap().name, "eeprom@50");

        // 验证 eeprom 已删除但 i2c@0 还在
        let i2c_node = root
            .find_child("soc")
            .unwrap()
            .find_child("i2c@0")
            .unwrap();
        assert!(i2c_node.find_child("eeprom@50").is_none());
    }

    #[test]
    fn test_remove_node_not_found() {
        let mut root = Node::root();

        // 测试删除不存在的节点
        let removed = root.remove_by_path("nonexistent").unwrap();
        assert!(removed.is_none(), "Should return None for nonexistent node");

        // 测试删除不存在的嵌套节点
        let removed = root.remove_by_path("soc/nonexistent").unwrap();
        assert!(
            removed.is_none(),
            "Should return None for nonexistent nested node"
        );
    }

    #[test]
    fn test_remove_node_invalid_path() {
        let mut root = Node::root();

        // 测试空路径
        let result = root.remove_by_path("");
        assert!(result.is_err(), "Should error for empty path");

        // 测试只有斜杠的路径
        let result = root.remove_by_path("/");
        assert!(result.is_err(), "Should error for root path");
    }

    #[test]
    fn test_fdt_remove_node() {
        let mut fdt = Fdt::new();

        // 添加根节点属性
        fdt.root
            .add_property(Property::address_cells(2))
            .add_property(Property::size_cells(2));

        // 添加子节点
        let mut soc = Node::new("soc");
        let mut gpio = Node::new("gpio@1000");
        gpio.add_property(Property::compatible_from_strs(&["gpio"]));
        soc.add_child(gpio);
        fdt.root.add_child(soc);

        // 验证节点存在
        assert!(fdt.get_by_path("/soc/gpio@1000").is_some());

        // 删除节点
        let removed = fdt.remove_node("soc/gpio@1000").unwrap();
        assert!(removed.is_some(), "Should remove gpio@1000");
        assert_eq!(removed.unwrap().name, "gpio@1000");

        // 验证节点已删除
        assert!(fdt.get_by_path("/soc/gpio@1000").is_none());
        assert!(fdt.get_by_path("/soc").is_some());
    }

    #[test]
    fn test_fdt_remove_node_with_alias() {
        let mut fdt = Fdt::new();

        // 添加根节点属性
        fdt.root
            .add_property(Property::address_cells(2))
            .add_property(Property::size_cells(2));

        // 添加别名节点
        let mut aliases = Node::new("aliases");
        aliases.add_property(Property::raw_string("serial0", "/soc/uart@1000"));
        fdt.root.add_child(aliases);

        // 添加 soc 和 uart 节点
        let mut soc = Node::new("soc");
        let mut uart = Node::new("uart@1000");
        uart.add_property(Property::compatible_from_strs(&["uart"]));
        soc.add_child(uart);
        fdt.root.add_child(soc);

        // 验证节点存在
        assert!(fdt.get_by_path("serial0").is_some());
        assert!(fdt.get_by_path("/soc/uart@1000").is_some());

        // 通过别名删除节点
        let removed = fdt.remove_node("serial0").unwrap();
        assert!(removed.is_some(), "Should remove uart via alias");
        assert_eq!(removed.unwrap().name, "uart@1000");

        // 验证节点已删除
        assert!(fdt.get_by_path("serial0").is_none());
        assert!(fdt.get_by_path("/soc/uart@1000").is_none());
    }

    #[test]
    fn test_remove_node_only_exact_matching() {
        let mut root = Node::root();

        // 添加相似名称的节点
        let mut gpio = Node::new("gpio@1000");
        let mut gpio_controller = Node::new("gpio-controller@2000");

        root.add_child(gpio);
        root.add_child(gpio_controller);

        // 测试精确删除：删除 gpio@1000
        let removed = root.remove_by_path("gpio").unwrap();
        // 由于只支持精确匹配，这应该找不到 gpio@1000
        assert!(
            removed.is_none(),
            "Should not find 'gpio' when only 'gpio@1000' exists"
        );

        // 精确删除 gpio@1000
        let removed = root.remove_by_path("gpio@1000").unwrap();
        assert!(removed.is_some(), "Should find and remove 'gpio@1000'");

        // 验证只有 gpio-controller@2000 还在
        assert!(root.find_child("gpio@1000").is_none());
        assert!(root.find_child("gpio-controller@2000").is_some());
    }
}
