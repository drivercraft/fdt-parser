#![cfg(unix)]

use dtb_file::*;
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
            if node.name().starts_with("memory") {
                found_memory = true;

                // 验证节点被识别为 NodeMemory 类型
                assert!(
                    node.as_memory().is_some(),
                    "Memory node should be detected as NodeMemory"
                );
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
            if let Some(mem) = node.as_memory() {
                let regions = mem.regions();
                // memory 节点应该有至少一个区域
                if !regions.is_empty() {
                    for region in &regions {
                        println!(
                            "Memory region: address=0x{:x}, size=0x{:x}",
                            region.address, region.size
                        );
                    }
                    // 验证总大小计算
                    let total = mem.total_size();
                    let expected: u64 = regions.iter().map(|r| r.size).sum();
                    assert_eq!(total, expected, "Total size should match sum of regions");
                }
            }
        }
    }

    #[test]
    fn test_memory_node_properties() {
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let Some(mem) = node.as_memory() {
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
        assert_eq!(mem.total_size(), 0, "Total size should be 0");
    }
}
