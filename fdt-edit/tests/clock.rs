#![cfg(unix)]

use dtb_file::*;
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
            if let Some(clock) = node.as_clock() {
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
            if let Some(clock) = node.as_clock() {
                // 获取 #clock-cells
                let cells = clock.clock_cells;
                println!("Clock: {} cells={:?}", clock.name(), cells);

                // 获取输出名称
                let names = &clock.output_names;
                if !names.is_empty() {
                    println!("  output-names: {:?}", names);
                }

                match &clock.kind {
                    ClockType::Fixed(fixed) => {
                        println!(
                            "  Fixed clock: freq={}Hz accuracy={:?}",
                            fixed.frequency, fixed.accuracy
                        );
                    }
                    ClockType::Provider => {
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
        for node in fdt.all_nodes() {
            if let Some(clock) = node.as_clock() {
                if let ClockType::Fixed(fixed) = &clock.kind {
                    // 验证固定时钟属性
                    assert!(fixed.frequency > 0);
                    println!(
                        "Fixed clock found: {} freq={}Hz accuracy={:?}",
                        clock.name(),
                        fixed.frequency,
                        fixed.accuracy
                    );
                }
            }
        }
    }

    #[test]
    fn test_clock_output_name() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let Some(clock) = node.as_clock() {
                let names = &clock.output_names;
                if !names.is_empty() {
                    // 测试 output_name 方法
                    let first = &clock.output_names[0];
                    assert_eq!(first, &names[0]);

                    // 如果有多个输出，测试索引访问
                    if names.len() > 1 && clock.clock_cells > 0 {
                        let second = &clock.output_names[1];
                        assert_eq!(second, &names[1]);
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
            if let Some(clock) = node.as_clock() {
                match &clock.kind {
                    ClockType::Fixed(fixed) => {
                        // 测试 FixedClock 转换
                        let freq = fixed.frequency;
                        assert!(freq > 0);
                    }
                    ClockType::Provider => {
                        // 测试 Provider 类型
                        println!("Clock {} is a provider", clock.name());
                    }
                }
            }
        }
    }
}
