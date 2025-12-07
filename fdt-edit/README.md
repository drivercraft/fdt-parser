# fdt-edit

用于创建、编辑和编码设备树（FDT）的高级 Rust 库。

## 概述

`fdt-edit` 是一个功能丰富的设备树操作库，基于 `fdt-raw` 构建，提供了完整的设备树创建、编辑和编码功能。该库支持从零创建新的设备树，修改现有的设备树，以及将编辑后的设备树编码为标准 DTB 格式。

## 特性

- **完整的设备树编辑**：支持节点和属性的增删改查
- **类型安全的节点操作**：提供专门的节点类型（时钟、内存、PCI、中断控制器等）
- **高效的编码器**：将内存中的设备树结构编码为标准 DTB 格式
- **phandle 管理**：自动 phandle 分配和引用管理
- **内存保留块支持**：完整的内存保留区域操作
- **`no_std` 兼容**：适用于嵌入式环境

## 核心组件

### Fdt 结构
可编辑的设备树容器：
- 从原始 DTB 数据解析
- 创建新的空设备树
- 管理 phandle 缓存
- 编码为 DTB 格式

### 节点系统
支持多种专用节点类型：
- **时钟节点**：时钟源和时钟消费者
- **内存节点**：内存区域定义
- **PCI 节点**：PCI 总线和设备
- **中断控制器**：中断映射和管理
- **通用节点**：可自定义的节点类型

### 属性系统
- **强类型属性**：各种数据类型的属性支持
- **自动属性管理**：智能的属性增删改查
- **格式化显示**：友好的节点和属性显示

## 快速开始

```rust
use fdt_edit::{Fdt, Node, NodeKind};

// 创建新的空设备树
let mut fdt = Fdt::new();

// 添加根节点下的子节点
let memory_node = fdt.root_mut()
    .add_child("memory@80000000")
    .unwrap();
memory_node.add_property("device_type", "memory")?;
memory_node.add_property("reg", &[0x8000_0000u64, 0x1000_0000u64])?;

// 添加时钟节点
let clock_node = fdt.root_mut()
    .add_child("clk_osc")
    .unwrap();
clock_node.add_property("compatible", &["fixed-clock"])?;
clock_node.add_property("#clock-cells", &[0u32])?;
clock_node.add_property("clock-frequency", &[24_000_000u32])?;

// 编码为 DTB 数据
let dtb_data = fdt.encode()?;
```

### 从现有 DTB 编辑

```rust
// 解析现有 DTB
let mut fdt = Fdt::from_bytes(&existing_dtb)?;

// 查找并修改节点
if let Some(cpu_node) = fdt.root_mut()
    .find_child_mut("cpus")?
    .and_then(|n| n.find_child_mut("cpu@0")) {

    // 修改时钟频率
    cpu_node.set_property("clock-frequency", &[1_200_000_000u32])?;
}

// 添加新的属性
cpu_node.add_property("new-property", "value")?;

// 重新编码
let modified_dtb = fdt.encode()?;
```

### 节点遍历和查找

```rust
// 遍历所有节点
for node in fdt.root().traverse() {
    match node.kind() {
        NodeKind::Memory(mem) => {
            println!("Memory node: {:x?}", mem.regions());
        }
        NodeKind::Clock(clock) => {
            println!("Clock: {}, freq: {}", clock.name(), clock.frequency()?);
        }
        _ => {
            println!("Generic node: {}", node.name());
        }
    }
}

// 查找特定节点
if let Some(chosen) = fdt.root().find_child("chosen") {
    if let Some(bootargs) = chosen.get_property("bootargs") {
        println!("Boot args: {}", bootargs.as_str()?);
    }
}
```

## 依赖

- `fdt-raw` - 底层 FDT 解析库
- `log = "0.4"` - 日志记录
- `enum_dispatch = "0.3.13"` - 枚举分发优化

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
4. 新功能添加相应的测试用例

## 相关项目

- [fdt-raw](../fdt-raw/) - 底层 FDT 解析库
- [fdt-parser](../fdt-parser/) - 高级缓存式 FDT 解析器
- [dtb-tool](../dtb-tool/) - DTB 文件检查工具
- [dtb-file](../dtb-file/) - 测试数据包

## 示例

更多使用示例请查看 `examples/` 目录（如果存在）或源码中的测试用例。