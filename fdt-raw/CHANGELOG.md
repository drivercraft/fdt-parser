# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/drivercraft/fdt-parser/compare/fdt-raw-v0.2.0...fdt-raw-v0.3.0) - 2026-03-09

### Added

- enhance FDT parser library with comprehensive improvements
- 更新版本号至0.1.5
- *(fdt)* [**breaking**] change memory method to return iterator for multiple nodes
- *(fdt)* change memory method to return iterator for multiple memory nodes
- *(node)* add compatibles method to retrieve compatible strings iterator
- 实现设备地址到CPU物理地址的转换功能，优化节点结构，更新版本号至0.1.2
- 添加路径查找节点和地址转换功能，优化节点结构
- 添加通过路径查找节点的方法，优化节点上下文结构，增强测试用例
- 更新版本号至 0.1.1，添加 Fdt 结构的 chosen 和 memory 方法，增强测试用例
- more doc
- 更新 Cargo.toml 和 README.md，添加元数据和文档链接；在 Bytes 结构中添加 is_empty 方法
- 添加 regs 方法以支持获取节点寄存器信息，并优化 RegIter 迭代器逻辑
- 移除不必要的上下文信息，简化属性迭代器和 reg 属性结构
- 更新属性处理，优化 reg 数据访问和增加新属性方法
- 添加 Chosen 和 Memory 节点结构，支持节点属性的查找和迭代
- 删除不再需要的示例代码和测试文件，优化代码结构
- 移除冗余的节点匹配和路径处理逻辑，简化代码结构
- 增加对节点路径查找和删除的支持，支持 node-name@unit-address 格式
- add fdt-edit ([#2](https://github.com/drivercraft/fdt-parser/pull/2))

### Fixed

- 修复 RegIter 迭代器中 address 变量的初始化方式

### Other

- Add inherited interrupt-parent lookup ([#10](https://github.com/drivercraft/fdt-parser/pull/10))
- Add convenience method for resolving chosen stdout node ([#9](https://github.com/drivercraft/fdt-parser/pull/9))
- improve iter ([#6](https://github.com/drivercraft/fdt-parser/pull/6))
- relax memory validation to handle incomplete test data
- fix code formatting for cargo fmt check
- 优化代码格式，清理冗余和不必要的换行
- Refactor FDT node handling and property management
- 优化 FDT 显示和调试测试，简化属性验证逻辑
- Refactor property handling in FDT
- 移除 FdtContext 结构体的默认实现，使用派生宏简化代码；优化 NodeRef 的导入
- Implement new property types and refactor existing property handling
