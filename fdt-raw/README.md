# fdt-raw

用于解析设备树二进制文件（DTB）的低级 Rust 库。

## 概述

`fdt-raw` 是一个基于 [Device Tree Specification v0.4](https://www.devicetree.org/specifications/) 实现的纯 Rust、`#![no_std]` 兼容的设备树解析库。该库提供了对扁平设备树（FDT）结构的底层访问接口，适用于嵌入式系统和裸机开发环境。

## 特性

- **纯 Rust 实现**：无需 C 语言依赖
- **`no_std` 兼容**：适用于裸机和嵌入式环境
- **基于规范**：严格遵循 Device Tree Specification v0.4
- **零拷贝解析**：直接在原始数据上操作，避免不必要的内存分配
- **类型安全**：提供强类型的 API 接口
- **内存高效**：使用 `heapless` 进行无分配器集合操作

## 核心组件

### Fdt 结构
主要的 FDT 解析器，提供对设备树结构的访问：
- 头部信息解析
- 内存保留块遍历
- 节点树遍历
- 属性访问

### 支持的节点类型
- **内存节点**：解析内存区域信息
- **chosen 节点**：访问启动参数
- **通用节点**：处理其他所有节点类型

### 属性解析
- **reg 属性**：地址范围解析，支持 `#address-cells` 和 `#size-cells`
- **属性迭代器**：高效的属性遍历
- **属性值访问**：提供各种数据类型的访问方法

## 快速开始

```rust
use fdt_raw::Fdt;

// 从字节数据解析 FDT
let fdt = Fdt::from_bytes(&dtb_data)?;

// 遍历根节点的子节点
for node in fdt.root().children() {
    println!("Node name: {}", node.name()?);

    // 遍历节点属性
    for prop in node.properties() {
        println!("  Property: {}", prop.name()?);
    }
}

// 访问内存保留块
for reservation in fdt.memory_reservations() {
    println!("Reserved: 0x{:x} - 0x{:x}",
             reservation.address,
             reservation.address + reservation.size);
}
```

## 依赖

- `heapless = "0.9"` - 无分配器集合
- `log = "0.4"` - 日志记录
- `thiserror = {version = "2", default-features = false}` - 错误处理

## 开发依赖

- `dtb-file` - 测试数据
- `env_logger = "0.11"` - 日志实现

## 许可证

本项目采用开源许可证，具体许可证类型请查看项目根目录的 LICENSE 文件。

## 贡献

欢迎提交 Issue 和 Pull Request。请确保：

1. 代码遵循项目的格式规范（`cargo fmt`）
2. 通过所有测试（`cargo test`）
3. 通过 Clippy 检查（`cargo clippy`）

## 相关项目

- [fdt-parser](../fdt-parser/) - 更高级的缓存式 FDT 解析器
- [fdt-edit](../fdt-edit/) - FDT 编辑和操作库
- [dtb-tool](../dtb-tool/) - DTB 文件检查工具
- [dtb-file](../dtb-file/) - 测试数据包