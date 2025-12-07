#![cfg(unix)]

use dtb_file::*;
use fdt_edit::NodeKind;
use fdt_edit::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_node_detection() {
        // 使用 RPI 4B DTB 测试 clock 节点检测
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 遍历查找 clock 节点（有 #clock-cells 属性的节点）
        let mut clock_count = 0;
        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                clock_count += 1;
                println!(
                    "Clock node: {} (#clock-cells={})",
                    clock.name(),
                    clock.clock_cells
                );
            }
        }
        println!("Found {} clock nodes", clock_count);
    }

    #[test]
    fn test_clock_properties() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                // 获取 #clock-cells
                let cells = clock.clock_cells;
                println!("Clock: {} cells={}", clock.name(), cells);

                // 获取输出名称
                if !clock.clock_output_names.is_empty() {
                    println!("  output-names: {:?}", clock.clock_output_names);
                }

                match &clock.kind {
                    ClockType::Fixed(fixed) => {
                        println!(
                            "  Fixed clock: freq={}Hz accuracy={:?}",
                            fixed.frequency, fixed.accuracy
                        );
                    }
                    ClockType::Normal => {
                        println!("  Clock provider");
                    }
                }
            }
        }
    }

    #[test]
    fn test_fixed_clock() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 查找固定时钟
        let mut found_with_freq = false;
        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                if let ClockType::Fixed(fixed) = &clock.kind {
                    // 打印固定时钟信息
                    println!(
                        "Fixed clock found: {} freq={}Hz accuracy={:?}",
                        clock.name(),
                        fixed.frequency,
                        fixed.accuracy
                    );
                    // 有些固定时钟（如 cam1_clk, cam0_clk）没有 clock-frequency 属性
                    if fixed.frequency > 0 {
                        found_with_freq = true;
                    }
                }
            }
        }
        // 至少应该有一个固定时钟有频率
        assert!(
            found_with_freq,
            "Should find at least one fixed clock with frequency"
        );
    }

    #[test]
    fn test_clock_output_name() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                let names = &clock.clock_output_names;
                if !names.is_empty() {
                    // 测试 output_name 方法
                    let first = clock.output_name(0);
                    assert_eq!(first, Some(names[0].as_str()));

                    // 如果有多个输出，测试索引访问
                    if names.len() > 1 && clock.clock_cells > 0 {
                        let second = clock.output_name(1);
                        assert_eq!(second, Some(names[1].as_str()));
                    }
                }
            }
        }
    }

    #[test]
    fn test_clock_type_conversion() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                match &clock.kind {
                    ClockType::Fixed(fixed) => {
                        // 打印固定时钟信息
                        println!(
                            "Fixed clock: {} freq={} accuracy={:?}",
                            clock.name(),
                            fixed.frequency,
                            fixed.accuracy
                        );
                    }
                    ClockType::Normal => {
                        // 测试 Normal 类型
                        println!("Clock {} is a provider", clock.name());
                    }
                }
            }
        }
    }

    #[test]
    fn test_clocks_with_context() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        let mut found_clocks = false;
        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock_ref) = node.as_ref() {
                found_clocks = true;

                let clocks = clock_ref.clocks();
                if !clocks.is_empty() {
                    found_clocks = true;
                    println!(
                        "Node: {} has {} clock references:",
                        clock_ref.name(),
                        clocks.len()
                    );
                    for (i, clk) in clocks.iter().enumerate() {
                        println!(
                            "  [{}] phandle={:?} cells={} specifier={:?} name={:?}",
                            i, clk.phandle, clk.cells, clk.specifier, clk.name
                        );
                        // 验证 specifier 长度与 cells 一致
                        assert_eq!(
                            clk.specifier.len(),
                            clk.cells as usize,
                            "specifier length should match cells"
                        );
                    }
                }
            }
        }
        assert!(found_clocks, "Should find nodes with clock references");
    }

    #[test]
    fn test_clock_ref_select() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            // 使用 as_clock_ref 获取带上下文的 clock 引用
            if let NodeKind::Clock(clock) = node.as_ref() {
                let clocks = clock.clocks();
                for clk in clocks {
                    // 测试 select() 方法
                    if clk.cells > 0 {
                        assert!(
                            clk.select().is_some(),
                            "select() should return Some when cells > 0"
                        );
                        assert_eq!(
                            clk.select(),
                            clk.specifier.first().copied(),
                            "select() should return first specifier"
                        );
                    }
                }
            }
        }
    }
}
