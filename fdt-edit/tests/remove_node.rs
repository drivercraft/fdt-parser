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
        // Parse original DTB
        let raw_data = fdt_qemu();
        let mut fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Find an existing node path to remove
        let node = fdt.get_by_path("/psci");
        assert!(node.is_some(), "psci node should exist");

        // Remove node
        let removed = fdt.remove_node("/psci");
        assert!(removed.is_ok(), "Removal should succeed");
        assert!(removed.unwrap().is_some(), "Should return the removed node");

        // Verify node has been removed
        let node_after = fdt.get_by_path("/psci");
        assert!(node_after.is_none(), "psci node should have been removed");
    }

    #[test]
    fn test_remove_node_exact_path_parts() {
        init_logging();
        // Parse original DTB
        let raw_data = fdt_qemu();
        let mut fdt = Fdt::from_bytes(&raw_data).unwrap();

        let memory = fdt.find_by_path("/memory").next().unwrap();
        fdt.remove_node(&memory.path()).unwrap();

        let cpus = fdt.find_by_path("/cpus/cpu").collect::<Vec<_>>();
        let path = cpus[0].path();
        println!("Removing node at path: {}", path);
        // drop(node);

        // Remove node
        let removed = fdt.remove_node(&path);
        assert!(removed.is_ok(), "Removal should succeed");
        assert!(removed.unwrap().is_some(), "Should return the removed node");

        // Verify node has been removed
        let node_after = fdt.get_by_path("/cpus/cpu@0");
        assert!(node_after.is_none(), "cpu node should have been removed");

        let raw = fdt.encode();
        let fdt2 = Fdt::from_bytes(&raw).unwrap();
        let node_after_reload = fdt2.get_by_path("/cpus/cpu@0");
        assert!(
            node_after_reload.is_none(),
            "cpu node should have been removed after reload"
        );
    }

    #[test]
    fn test_remove_nested_node() {
        // Use manually created tree to test nested removal
        let mut fdt = Fdt::new();

        // Create nested nodes: /soc/i2c@0/eeprom@50
        let mut soc = Node::new("soc");
        let mut i2c = Node::new("i2c@0");
        let eeprom = Node::new("eeprom@50");
        i2c.add_child(eeprom);
        soc.add_child(i2c);
        fdt.root.add_child(soc);

        // Verify node exists
        assert!(fdt.get_by_path("/soc/i2c@0/eeprom@50").is_some());

        // Remove nested node
        let removed = fdt.remove_node("/soc/i2c@0/eeprom@50");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // Verify node has been removed
        assert!(fdt.get_by_path("/soc/i2c@0/eeprom@50").is_none());

        // Parent nodes should still exist
        assert!(fdt.get_by_path("/soc/i2c@0").is_some());
        assert!(fdt.get_by_path("/soc").is_some());
    }

    #[test]
    fn test_remove_nonexistent_node() {
        let mut fdt = Fdt::new();

        // Removing non-existent node should return NotFound
        let result = fdt.remove_node("/nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_direct_child() {
        let mut fdt = Fdt::new();

        // Add direct child node
        fdt.root.add_child(Node::new("memory@0"));

        // Verify it exists
        assert!(fdt.get_by_path("/memory@0").is_some());

        // Remove direct child node
        let removed = fdt.remove_node("/memory@0");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // Verify it has been removed
        assert!(fdt.get_by_path("/memory@0").is_none());
    }

    #[test]
    fn test_remove_empty_path() {
        let mut fdt = Fdt::new();

        // Empty path should return error
        let result = fdt.remove_node("");
        assert!(result.is_err());

        let result = fdt.remove_node("/");
        assert!(result.is_err());
    }

    #[test]
    fn test_node_remove_by_path() {
        // Test Node's remove_by_path method directly
        let mut root = Node::new("");

        // Create structure: /a/b/c
        let mut a = Node::new("a");
        let mut b = Node::new("b");
        let c = Node::new("c");
        b.add_child(c);
        a.add_child(b);
        root.add_child(a);

        // Verify c exists
        assert!(root.get_child("a").is_some());

        // Remove c
        let removed = root.remove_by_path("a/b/c");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // Remove b
        let removed = root.remove_by_path("a/b");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // Remove a
        let removed = root.remove_by_path("a");
        assert!(removed.is_ok());
        assert!(removed.unwrap().is_some());

        // All nodes have been removed
        assert!(root.get_child("a").is_none());
    }

    #[test]
    fn test_remove_with_leading_slash() {
        let mut fdt = Fdt::new();
        let node = fdt.root_mut().add_child(Node::new("test"));
        assert_eq!(&node.path(), "/test");
        println!("Node:\n {:?}", node);

        // Both paths with and without leading slash should work
        let result = fdt.remove_node("/test");
        assert!(result.is_ok());

        assert!(fdt.get_by_path("/test").is_none());
    }

    #[test]
    fn test_remove_node_preserves_siblings() {
        let mut fdt = Fdt::new();

        // Add multiple sibling nodes
        fdt.root.add_child(Node::new("node1"));
        fdt.root.add_child(Node::new("node2"));
        fdt.root.add_child(Node::new("node3"));

        // Remove middle node
        let removed = fdt.remove_node("/node2");
        assert!(removed.is_ok());

        // Verify other nodes still exist
        assert!(fdt.get_by_path("/node1").is_some());
        assert!(fdt.get_by_path("/node2").is_none());
        assert!(fdt.get_by_path("/node3").is_some());
    }
}
