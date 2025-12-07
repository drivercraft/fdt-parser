//! FDT 深度调试演示
//!
//! 演示如何使用新的深度调试功能来遍历和打印设备树的所有节点

use dtb_file::fdt_rpi_4b;
use fdt_edit::Fdt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // 从 RPI 4B DTB 数据创建 FDT
    let raw_data = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw_data)?;

    println!("=== FDT 基本调试信息 ===");
    // 基本调试格式（紧凑）
    println!("{:?}", fdt);
    println!();

    println!("=== FDT 深度调试信息 ===");
    // 深度调试格式（遍历所有节点）
    println!("{:#?}", fdt);

    Ok(())
}
