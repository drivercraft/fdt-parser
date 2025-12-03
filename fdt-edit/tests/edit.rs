#![cfg(unix)]

use dtb_file::*;
use fdt_edit::*;

#[test]
fn test_parse_and_rebuild() {
    // 解析原始 DTB
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    // 验证根节点
    assert!(fdt.root.name.is_empty(), "root node should have empty name");

    // 验证有属性
    assert!(
        !fdt.root.properties.is_empty(),
        "root should have properties"
    );

    // 验证有子节点
    assert!(!fdt.root.children.is_empty(), "root should have children");

    // 查找 memory 节点
    let has_memory = fdt
        .root
        .children
        .values()
        .any(|c| c.name.starts_with("memory"));
    assert!(has_memory, "should have memory node");

    // 重新序列化
    let rebuilt = fdt.to_bytes();

    // 验证重建后的数据可以被重新解析
    let reparsed = Fdt::from_bytes(&rebuilt).unwrap();

    // 验证基本结构一致
    assert_eq!(fdt.root.children.len(), reparsed.root.children.len());
    assert_eq!(fdt.root.properties.len(), reparsed.root.properties.len());
}
