#![cfg(unix)]

use dtb_file::*;
use fdt_edit::NodeKind;
use fdt_edit::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_node_detection() {
        // 使用 phytium DTB 测试 memory 节点检测
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 遍历查找 memory 节点
        let mut found_memory = false;
        for node in fdt.all_nodes() {
            if let NodeKind::Memory(mem) = node.as_ref() {
                found_memory = true;
                println!("Memory node: {}", mem.name());
            }
        }
        assert!(found_memory, "Should find at least one memory node");
    }

    #[test]
    fn test_memory_regions() {
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 查找 memory 节点并获取区域信息
        for node in fdt.all_nodes() {
            if let NodeKind::Memory(mem) = node.as_ref() {
                let regions = mem.regions();
                // memory 节点应该有至少一个区域
                if !regions.is_empty() {
                    for region in regions {
                        println!(
                            "Memory region: address=0x{:x}, size=0x{:x}",
                            region.address, region.size
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_memory_node_properties() {
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Memory(mem) = node.as_ref() {
                // memory 节点应该有 device_type 属性
                let dt = mem.device_type();
                if let Some(device_type) = dt {
                    assert_eq!(device_type, "memory", "device_type should be 'memory'");
                }

                // 获取节点名称
                let name = mem.name();
                assert!(
                    name.starts_with("memory"),
                    "Memory node name should start with 'memory'"
                );
            }
        }
    }

    #[test]
    fn test_create_memory_node() {
        // 手动创建一个 memory 节点
        let mem = NodeMemory::new("memory@80000000");
        assert_eq!(mem.name(), "memory@80000000");

        // 验证初始状态
        assert!(
            mem.regions().is_empty(),
            "New memory node should have no regions"
        );
    }
}
