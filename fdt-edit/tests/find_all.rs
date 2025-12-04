#[cfg(test)]
mod tests {
    use fdt_edit::Property;
    use fdt_edit::*;

    #[test]
    fn test_find_all_method() {
        let mut root = Node::root();

        // 添加几个子节点
        let mut gpio0 = Node::new_raw("gpio@1000");
        gpio0.add_property(Property::compatible_from_strs(&["gpio"]));

        let mut gpio1 = Node::new_raw("gpio@2000");
        gpio1.add_property(Property::compatible_from_strs(&["gpio"]));

        let mut uart = Node::new_raw("uart@3000");
        uart.add_property(Property::compatible_from_strs(&["uart"]));

        root.add_child(gpio0);
        root.add_child(gpio1);
        root.add_child(uart);

        // 测试 find_all 与前缀匹配
        let gpio_nodes = root.find_all("gpio");
        assert_eq!(gpio_nodes.len(), 2); // 应该找到两个 GPIO 节点

        // 验证返回的路径是正确的
        for (node, path) in &gpio_nodes {
            assert!(path.starts_with("/gpio@"));
            assert!(node.name.starts_with("gpio"));
        }
    }

    #[test]
    fn test_find_all_nested() {
        let mut root = Node::root();

        // 创建嵌套结构
        let mut soc = Node::new_raw("soc");

        let mut i2c0 = Node::new_raw("i2c@0");
        let mut eeprom = Node::new_raw("eeprom@50");
        eeprom.add_property(Property::compatible_from_strs(&["eeprom"]));

        let mut i2c1 = Node::new_raw("i2c@1");
        let mut sensor = Node::new_raw("sensor@60");
        sensor.add_property(Property::compatible_from_strs(&["sensor"]));

        i2c0.add_child(eeprom);
        i2c1.add_child(sensor);
        soc.add_child(i2c0);
        soc.add_child(i2c1);
        root.add_child(soc);

        // 测试嵌套查找
        let i2c_nodes = root.find_all("soc/i2c");
        assert_eq!(i2c_nodes.len(), 2); // 应该找到 i2c@0 和 i2c@1 节点

        let eeprom_nodes = root.find_all("soc/i2c@0/eeprom");
        assert_eq!(eeprom_nodes.len(), 1); // 应该找到 eeprom@50 节点
    }

    #[test]
    fn test_find_all_prefix_matching() {
        let mut root = Node::root();

        // 添加多个带地址的节点
        let gpio0 = Node::new_raw("gpio@1000");
        let gpio1 = Node::new_raw("gpio@2000");
        let gpio2 = Node::new_raw("gpio-controller@3000");
        let uart0 = Node::new_raw("uart@4000");
        let uart1 = Node::new_raw("serial@5000");

        root.add_child(gpio0);
        root.add_child(gpio1);
        root.add_child(gpio2);
        root.add_child(uart0);
        root.add_child(uart1);

        // 测试前缀匹配：查找所有以 "gpio" 开头的节点
        let gpio_nodes = root.find_all("gpio");
        assert_eq!(gpio_nodes.len(), 3); // 应该找到所有 gpio 开头的节点

        // 验证找到的节点名称
        for (node, path) in &gpio_nodes {
            assert!(
                node.name.starts_with("gpio"),
                "Node '{}' should start with 'gpio'",
                node.name
            );
            assert_eq!(path.as_str(), format!("/{}", node.name));
        }

        // 测试前缀匹配：查找所有以 "uart" 开头的节点
        let uart_nodes = root.find_all("uart");
        assert_eq!(uart_nodes.len(), 1); // 应该找到 uart@4000，但不会找到 serial@5000

        // 测试精确匹配
        let exact_uart = root.find_all("uart@4000");
        assert_eq!(exact_uart.len(), 1); // 精确匹配应该找到 uart@4000

        // 测试前缀匹配 "serial"
        let serial_nodes = root.find_all("serial");
        assert_eq!(serial_nodes.len(), 1); // 应该找到 serial@5000
    }

    #[test]
    fn test_find_all_intermediate_exact_matching() {
        let mut root = Node::root();

        // 创建嵌套结构，测试中间级别的精确匹配
        let mut soc = Node::new_raw("soc");
        let mut bus = Node::new_raw("bus");

        let device1 = Node::new_raw("device@1000");
        let device2 = Node::new_raw("device@2000");

        bus.add_child(device1);
        bus.add_child(device2);
        soc.add_child(bus);
        root.add_child(soc);

        // 测试中间级别必须精确匹配：soc/bus 是正确的路径
        let devices = root.find_all("soc/bus/device");
        assert_eq!(devices.len(), 2); // 应该找到两个 device 节点

        // 测试中间级别模糊匹配不应该工作：soc/b 不会匹配 bus
        let no_match = root.find_all("soc/b/device");
        assert_eq!(no_match.len(), 0); // 中间级别的模糊匹配不应该工作
    }
}
