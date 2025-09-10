use dtb_file::fdt_rpi_4b;
use fdt_parser::Fdt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = fdt_rpi_4b();
    // 假设我们有一个设备树数据
    let fdt = Fdt::from_bytes(&raw)?;

    let mut all = vec![];

    println!("=== 递归遍历所有节点 ===");
    fdt.walk_nodes_recursive(|node| {
        all.push(node.clone());
        Ok(true) // 继续遍历
    })?;

    for node in &all {
        println!(
            "{}└─ {} (level: {})",
            "  ".repeat(node.level()),
            node.name(),
            node.level()
        );
    }

    println!("\n=== 只遍历根级别节点 (depth 0) ===");
    fdt.walk_nodes_at_depth(0, |node| {
        println!("Root node: {}", node.name());
        Ok(true)
    })?;

    println!("\n=== 只遍历第二级节点 (depth 1) ===");
    fdt.walk_nodes_at_depth(1, |node| {
        println!("Level 1 node: {}", node.name());
        Ok(true)
    })?;

    println!("\n=== 查找特定节点的子节点 ===");
    // 查找名为 "soc" 的节点的子节点
    fdt.walk_child_nodes("soc", 1, |node| {
        println!("Child of 'soc': {}", node.name());
        Ok(true)
    })?;

    println!("\n=== 使用 Node 的 walk_children 方法 ===");
    // 先找到一个节点，然后遍历其子节点
    fdt.walk_nodes_recursive(|node| {
        if node.name() == "soc" {
            println!("Found 'soc' node, traversing its children:");
            node.walk_children(|child| {
                println!("  Child: {}", child.name());
                Ok(true)
            })?;
            Ok(false) // 找到后停止遍历
        } else {
            Ok(true) // 继续查找
        }
    })?;

    println!("\n=== 条件遍历示例 ===");
    let mut count = 0;
    fdt.walk_nodes_recursive(|node| {
        count += 1;
        println!("Node #{}: {} (level: {})", count, node.name(), node.level());

        // 只遍历前10个节点
        if count >= 10 {
            Ok(false)
        } else {
            Ok(true)
        }
    })?;

    Ok(())
}
