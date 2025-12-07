#![cfg(unix)]

use dtb_file::*;
use fdt_edit::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fdt_display() {
        // 使用 RPI 4B DTB 测试 Display 功能
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 测试 Display 输出
        let dts_output = format!("{}", fdt);

        // 验证输出包含 DTS 头部
        assert!(dts_output.contains("/dts-v1/;"));

        // 验证输出包含根节点
        assert!(dts_output.contains("/ {"));

        println!("FDT Display output:\n{}", dts_output);
    }

    #[test]
    fn test_fdt_debug() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 测试 Debug 输出
        let debug_output = format!("{:?}", fdt);

        // 验证 Debug 输出包含结构体信息
        assert!(debug_output.contains("Fdt"));
        assert!(debug_output.contains("boot_cpuid_phys"));

        println!("FDT Debug output:\n{}", debug_output);
    }

    #[test]
    fn test_node_display() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 找到一个节点进行测试
        for node in fdt.all_nodes() {
            if node.name().contains("gpio") {
                let dts_output = format!("{}", node);

                // 验证输出包含节点名称
                assert!(dts_output.contains("gpio"));

                println!("Node Display output:\n{}", dts_output);
                break;
            }
        }
    }

    #[test]
    fn test_node_debug() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if node.name().contains("gpio") {
                let debug_output = format!("{:?}", node);

                // 验证 Debug 输出包含 Node 结构体信息
                assert!(debug_output.contains("NodeRef"));
                assert!(debug_output.contains("name"));

                println!("Node Debug output:\n{}", debug_output);
                break;
            }
        }
    }

    #[test]
    fn test_clock_node_display() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                let display_output = format!("{}", clock);
                let debug_output = format!("{:?}", clock);

                println!("Clock Node Display:\n{}", display_output);
                println!("Clock Node Debug:\n{}", debug_output);

                // 验证输出包含时钟相关信息
                assert!(display_output.contains("Clock Node"));

                // 验证 Debug 包含详细信息
                assert!(debug_output.contains("NodeRefClock"));
                assert!(debug_output.contains("clock_cells"));

                break;
            }
        }
    }

    #[test]
    fn test_interrupt_controller_display() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::InterruptController(ic) = node.as_ref() {
                let display_output = format!("{}", ic);
                let debug_output = format!("{:?}", ic);

                println!("Interrupt Controller Display:\n{}", display_output);
                println!("Interrupt Controller Debug:\n{}", debug_output);

                // 验证输出包含中断控制器相关信息
                assert!(display_output.contains("Interrupt Controller"));

                // 验证 Debug 包含详细信息
                assert!(debug_output.contains("NodeRefInterruptController"));
                assert!(debug_output.contains("interrupt_cells"));

                break;
            }
        }
    }

    #[test]
    fn test_memory_node_display() {
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Memory(mem) = node.as_ref() {
                let display_output = format!("{}", mem);
                let debug_output = format!("{:?}", mem);

                println!("Memory Node Display:\n{}", display_output);
                println!("Memory Node Debug:\n{}", debug_output);

                // 验证输出包含内存相关信息
                assert!(display_output.contains("Memory Node"));

                // 验证 Debug 包含详细信息
                assert!(debug_output.contains("NodeRefMemory"));
                assert!(debug_output.contains("regions_count"));

                break;
            }
        }
    }

    #[test]
    fn test_noderef_display_with_details() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if node.name().contains("clock") {
                let display_output = format!("{}", node);

                println!("NodeRef Display with details:\n{}", display_output);

                // 验证输出包含类型信息
                assert!(display_output.contains("Clock Node"));

                break;
            }
        }
    }

    #[test]
    fn test_create_simple_fdt() {
        let mut fdt = Fdt::new();

        // 测试基本 Display 功能
        let dts_output = format!("{}", fdt);
        println!("Created FDT Display:\n{}", dts_output);

        // 验证输出包含基本头部
        assert!(dts_output.contains("/dts-v1/;"));
        assert!(dts_output.contains("/ {"));
    }

    #[test]
    fn test_manual_node_display() {
        let mut node = Node::new("test-node");

        // 测试基本 Display 功能
        let display_output = format!("{}", node);
        println!("Manual Node Display:\n{}", display_output);

        // 验证输出包含节点名称
        assert!(display_output.contains("test-node"));

        // 测试 Debug
        let debug_output = format!("{:?}", node);
        println!("Manual Node Debug:\n{}", debug_output);

        assert!(debug_output.contains("Node"));
        assert!(debug_output.contains("test-node"));
    }

    #[test]
    fn test_fdt_deep_debug() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 测试基本 Debug 格式
        let simple_debug = format!("{:?}", fdt);
        println!("FDT Simple Debug:\n{}", simple_debug);

        // 验证基本格式包含基本信息
        assert!(simple_debug.contains("Fdt"));
        assert!(simple_debug.contains("boot_cpuid_phys"));

        // 测试深度 Debug 格式
        let deep_debug = format!("{:#?}", fdt);
        println!("FDT Deep Debug:\n{}", deep_debug);

        // 验证深度格式包含节点信息
        assert!(deep_debug.contains("Fdt {"));
        assert!(deep_debug.contains("nodes:"));
        assert!(deep_debug.contains("[000]"));

        // 验证包含特定节点类型
        assert!(
            deep_debug.contains("Clock")
                || deep_debug.contains("InterruptController")
                || deep_debug.contains("Memory")
                || deep_debug.contains("Generic")
        );
    }

    #[test]
    fn test_fdt_deep_debug_with_simple_tree() {
        let mut fdt = Fdt::new();

        // 创建一个简单的树结构进行测试
        let mut soc = Node::new("soc");
        soc.set_property(Property::new("#address-cells", vec![0x1, 0x0, 0x0, 0x0]));
        soc.set_property(Property::new("#size-cells", vec![0x1, 0x0, 0x0, 0x0]));

        let mut uart = Node::new("uart@9000000");
        uart.set_property(Property::new("compatible", b"arm,pl011\0".to_vec()));
        uart.set_property(Property::new(
            "reg",
            vec![
                0x00, 0x90, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
            ],
        ));
        uart.set_property(Property::new("status", b"okay\0".to_vec()));

        soc.add_child(uart);
        fdt.root.add_child(soc);

        // 测试深度调试输出
        let deep_debug = format!("{:#?}", fdt);
        println!("Simple Tree Deep Debug:\n{}", deep_debug);

        // 验证输出包含预期的节点信息
        assert!(deep_debug.contains("[000] : Generic"));
        assert!(deep_debug.contains("[001] soc: Generic"));
        assert!(deep_debug.contains("[002] uart@9000000: Generic"));
        assert!(deep_debug.contains("#address-cells=1"));
        assert!(deep_debug.contains("#size-cells=1"));
    }
}
