#[cfg(test)]
mod tests {
    use std::sync::Once;

    use dtb_file::fdt_qemu;
    use fdt_edit::*;

    fn init_logging() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let _ = env_logger::builder()
                .is_test(true)
                .filter_level(log::LevelFilter::Trace)
                .try_init();
        });
    }

    #[test]
    fn test_remove_node_exact_path() {
        init_logging();
        // 解析原始 DTB
        let raw_data = fdt_qemu();
        let mut fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 找到一个存在的节点路径进行删除
        let node = fdt.get_by_path("/psci");
        assert!(node.is_some(), "psci 节点应该存在");

        // 删除节点
        let removed = fdt.remove_node("/psci");
        assert!(removed.is_ok(), "删除应该成功");
        assert!(removed.unwrap().is_some(), "应该返回被删除的节点");

        // 验证节点已被删除
        let node_after = fdt.get_by_path("/psci");
        assert!(node_after.is_none(), "psci 节点应该已被删除");
    }

    #[test]
    fn test_remove_node_exact_path_parts() {
        init_logging();
        // 解析原始 DTB
        let raw_data = fdt_qemu();
        let mut fdt = Fdt::from_bytes(&raw_data).unwrap();

        let memory = fdt.find_by_path("/memory").next().unwrap();
        fdt.remove_node(&memory.path()).unwrap();

        let cpus = fdt.find_by_path("/cpus/cpu").collect::<Vec<_>>();
        let path = cpus[0].path();
        println!("Removing node at path: {}", path);
        // drop(node);

        // 删除节点
        let removed = fdt.remove_node(&path);
        assert!(removed.is_ok(), "删除应该成功");
        assert!(removed.unwrap().is_some(), "应该返回被删除的节点");

        // 验证节点已被删除
        let node_after = fdt.get_by_path("/cpus/cpu@0");
        assert!(node_after.is_none(), "cpu 节点应该已被删除");

        let raw = fdt.encode();
        let fdt2 = Fdt::from_bytes(&raw).unwrap();
        let node_after_reload = fdt2.get_by_path("/cpus/cpu@0");
        assert!(
            node_after_reload.is_none(),
            "重新加载后 cpu 节点应该已被删除"
        );
    }

    #[test]
    fn test_remove_nested_node() {
        // 使用手动创建的树测试嵌套删除
        let mut fdt = Fdt::new();

        // 创建嵌套节点: /soc/i2c@0/eeprom@50
        let mut soc = Node::new("soc");
        let mut i2c = Node::new("i2c@0");
        let eeprom = Node::new("eeprom@50");
        i2c.add_child(eeprom);
        soc.add_child(i2c);
        fdt.root.add_child(soc);

        // 验证节点存在
        assert!(fdt.get_by_path("/soc/i2c@0/eeprom@50").is_some());

        // 删除嵌套节点
        let removed = fdt.remove_node("/soc/i2c@0/eeprom@50");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // 验证节点已删除
        assert!(fdt.get_by_path("/soc/i2c@0/eeprom@50").is_none());

        // 父节点应该仍然存在
        assert!(fdt.get_by_path("/soc/i2c@0").is_some());
        assert!(fdt.get_by_path("/soc").is_some());
    }

    #[test]
    fn test_remove_nonexistent_node() {
        let mut fdt = Fdt::new();

        // 删除不存在的节点应该返回 NotFound
        let result = fdt.remove_node("/nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_direct_child() {
        let mut fdt = Fdt::new();

        // 添加直接子节点
        fdt.root.add_child(Node::new("memory@0"));

        // 验证存在
        assert!(fdt.get_by_path("/memory@0").is_some());

        // 删除直接子节点
        let removed = fdt.remove_node("/memory@0");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // 验证已删除
        assert!(fdt.get_by_path("/memory@0").is_none());
    }

    #[test]
    fn test_remove_empty_path() {
        let mut fdt = Fdt::new();

        // 空路径应该返回错误
        let result = fdt.remove_node("");
        assert!(result.is_err());

        let result = fdt.remove_node("/");
        assert!(result.is_err());
    }

    #[test]
    fn test_node_remove_by_path() {
        // 直接测试 Node 的 remove_by_path 方法
        let mut root = Node::new("");

        // 创建结构: /a/b/c
        let mut a = Node::new("a");
        let mut b = Node::new("b");
        let c = Node::new("c");
        b.add_child(c);
        a.add_child(b);
        root.add_child(a);

        // 验证 c 存在
        assert!(root.get_child("a").is_some());

        // 删除 c
        let removed = root.remove_by_path("a/b/c");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // 删除 b
        let removed = root.remove_by_path("a/b");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // 删除 a
        let removed = root.remove_by_path("a");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // 所有节点都已删除
        assert!(root.get_child("a").is_none());
    }

    #[test]
    fn test_remove_with_leading_slash() {
        let mut fdt = Fdt::new();
        let node = fdt.root_mut().add_child(Node::new("test"));
        assert_eq!(&node.path(), "/test");
        println!("Node:\n {:?}", node);

        // 带有和不带斜杠的路径都应该工作
        let result = fdt.remove_node("/test");
        assert!(result.is_ok());

        assert!(fdt.get_by_path("/test").is_none());
    }

    #[test]
    fn test_remove_node_preserves_siblings() {
        let mut fdt = Fdt::new();

        // 添加多个兄弟节点
        fdt.root.add_child(Node::new("node1"));
        fdt.root.add_child(Node::new("node2"));
        fdt.root.add_child(Node::new("node3"));

        // 删除中间节点
        let removed = fdt.remove_node("/node2");
        assert!(removed.is_ok());

        // 验证其他节点仍然存在
        assert!(fdt.get_by_path("/node1").is_some());
        assert!(fdt.get_by_path("/node2").is_none());
        assert!(fdt.get_by_path("/node3").is_some());
    }
}
