#![cfg(unix)]

use dtb_file::*;
use fdt_edit::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_controller_detection() {
        // 使用 RPI 4B DTB 测试中断控制器节点检测
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 遍历查找中断控制器节点
        let mut irq_count = 0;
        for node in fdt.all_nodes() {
            if let Node::InterruptController(ic) = node.as_ref() {
                irq_count += 1;
                println!(
                    "Interrupt controller: {} (#interrupt-cells={:?})",
                    ic.name(),
                    ic.interrupt_cells()
                );
            }
        }
        println!("Found {} interrupt controllers", irq_count);
        assert!(irq_count > 0, "Should find at least one interrupt controller");
    }

    #[test]
    fn test_interrupt_controller_properties() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let Node::InterruptController(ic) = node.as_ref() {
                // 获取 #interrupt-cells
                let cells = ic.interrupt_cells();
                println!("IRQ Controller: {} cells={:?}", ic.name(), cells);

                // 获取 #address-cells (如果有)
                let addr_cells = ic.interrupt_address_cells();
                if addr_cells.is_some() {
                    println!("  #address-cells: {:?}", addr_cells);
                }

                // 验证 is_interrupt_controller
                assert!(
                    ic.is_interrupt_controller(),
                    "Should be marked as interrupt controller"
                );

                // 获取 compatible 列表
                let compat = ic.compatibles();
                if !compat.is_empty() {
                    println!("  compatible: {:?}", compat);
                }
            }
        }
    }

    #[test]
    fn test_interrupt_controller_by_name() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // 查找 GIC (ARM Generic Interrupt Controller)
        let mut found_gic = false;
        for node in fdt.all_nodes() {
            if let Node::InterruptController(ic) = node.as_ref() {
                let compat = ic.compatibles();
                if compat.iter().any(|c| c.contains("gic")) {
                    found_gic = true;
                    println!("Found GIC: {}", ic.name());

                    // GIC 通常有 3 个 interrupt-cells
                    let cells = ic.interrupt_cells();
                    println!("  #interrupt-cells: {:?}", cells);
                }
            }
        }
        // 注意：并非所有 DTB 都有 GIC，这里只是示例
        if found_gic {
            println!("GIC found in this DTB");
        }
    }

    #[test]
    fn test_interrupt_controller_with_phytium() {
        // Phytium DTB 应该有中断控制器
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        let mut controllers = Vec::new();
        for node in fdt.all_nodes() {
            if let Node::InterruptController(ic) = node.as_ref() {
                controllers.push((
                    ic.name().to_string(),
                    ic.interrupt_cells(),
                    ic.compatibles().join(", "),
                ));
            }
        }

        println!("Interrupt controllers in Phytium DTB:");
        for (name, cells, compat) in &controllers {
            println!("  {} (#interrupt-cells={:?}, compatible={})", name, cells, compat);
        }

        assert!(
            !controllers.is_empty(),
            "Phytium should have at least one interrupt controller"
        );
    }

    #[test]
    fn test_interrupt_controller_detection_logic() {
        // 测试节点是否正确被识别为中断控制器
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            let name = node.name();
            let is_ic = matches!(node.as_ref(), Node::InterruptController(_));

            // 如果节点名以 interrupt-controller 开头，应该被识别
            if name.starts_with("interrupt-controller") && !is_ic {
                println!(
                    "Warning: {} might be an interrupt controller but not detected",
                    name
                );
            }

            // 如果有 interrupt-controller 属性，应该被识别
            if node.find_property("interrupt-controller").is_some() && !is_ic {
                println!(
                    "Warning: {} has interrupt-controller property but not detected",
                    name
                );
            }
        }
    }

    #[test]
    fn test_interrupt_cells_values() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let Node::InterruptController(ic) = node.as_ref() {
                if let Some(cells) = ic.interrupt_cells() {
                    // 常见的 interrupt-cells 值：1, 2, 3
                    assert!(
                        cells >= 1 && cells <= 4,
                        "Unusual #interrupt-cells value: {} for {}",
                        cells,
                        ic.name()
                    );
                }
            }
        }
    }
}
