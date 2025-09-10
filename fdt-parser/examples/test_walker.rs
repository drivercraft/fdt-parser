use dtb_file::fdt_rpi_4b;
use fdt_parser::Fdt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用设备树数据
    let dtb_data = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&dtb_data)?;

    println!("=== 测试 Walker 结构体 ===");
    
    // 创建Walker实例
    let walker = fdt.walker();
    
    // 1. 测试walk_all - 遍历所有节点
    println!("1. 前10个节点:");
    let mut count = 0;
    walker.walk_all(|node| {
        if count < 10 {
            println!("  {}├─ {} (level: {})", 
                    "  ".repeat(node.level()), 
                    node.name(), 
                    node.level());
            count += 1;
            Ok(true)
        } else {
            Ok(false) // 停止遍历
        }
    })?;

    // 2. 测试count_nodes - 计算总节点数
    let total_count = walker.count_nodes()?;
    println!("\n2. 总节点数: {}", total_count);

    // 3. 测试walk_at_depth - 遍历指定深度
    println!("\n3. 第1层节点:");
    walker.walk_at_depth(1, |node| {
        println!("  Level 1: {}", node.name());
        Ok(true)
    })?;

    // 4. 测试count_nodes_at_depth - 计算指定深度节点数
    let level1_count = walker.count_nodes_at_depth(1)?;
    println!("\n4. 第1层节点数: {}", level1_count);

    // 5. 测试find_node - 查找特定节点
    println!("\n5. 查找 'soc' 节点:");
    let found = walker.find_node("soc", |node| {
        println!("  找到: {} (level: {})", node.name(), node.level());
        Ok(())
    })?;
    println!("  找到结果: {}", found);

    // 6. 测试walk_children - 遍历子节点
    println!("\n6. soc节点的前10个子节点:");
    let mut child_count = 0;
    walker.walk_children("soc", 1, |node| {
        if child_count < 10 {
            println!("  Child: {}", node.name());
            child_count += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    })?;

    // 7. 测试find_nodes - 查找所有匹配条件的节点
    println!("\n7. 查找所有名称包含'gpio'的节点:");
    let gpio_count = walker.find_nodes(
        |node| node.name().contains("gpio"),
        |node| {
            println!("  GPIO节点: {} (level: {})", node.name(), node.level());
            Ok(())
        }
    )?;
    println!("  找到 {} 个GPIO相关节点", gpio_count);

    // 8. 测试walk_until - 遍历直到满足条件
    println!("\n8. 遍历直到找到 'cpu@0' 节点:");
    let found_cpu = walker.walk_until(
        |node| node.name() == "cpu@0",
        |node| {
            println!("  遍历: {} (level: {})", node.name(), node.level());
            Ok(())
        }
    )?;
    println!("  找到cpu@0: {}", found_cpu);

    // 9. 测试walk_descendants - 遍历后代节点
    println!("\n9. gpio@7e200000节点的前5个后代:");
    let mut desc_count = 0;
    walker.walk_descendants("gpio@7e200000", 2, |node| {
        if desc_count < 5 {
            println!("  Descendant: {} (level: {})", node.name(), node.level());
            desc_count += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    })?;

    println!("\n=== Walker 测试完成 ===");
    Ok(())
}
