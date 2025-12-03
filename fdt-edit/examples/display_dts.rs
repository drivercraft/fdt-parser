use core::fmt::Write;
use fdt_edit::{Fdt, Node, Property, RegEntry};

fn main() {
    let mut fdt = Fdt::new();

    // 设置根节点属性
    fdt.root.add_property(Property::AddressCells(1));
    fdt.root.add_property(Property::SizeCells(1));
    fdt.root
        .add_property(Property::Compatible(vec!["vendor,board".to_string()]));
    fdt.root
        .add_property(Property::Model("Demo Board".to_string()));

    // 添加内存保留块
    fdt.memory_reservations.push(fdt_edit::MemoryReservation {
        address: 0x80000000,
        size: 0x200000,
    });

    // 添加 CPU 节点
    let mut cpu0 = Node::new("cpu@0");
    cpu0.add_property(Property::DeviceType("cpu".to_string()));
    cpu0.add_property(Property::Compatible(vec!["arm,cortex-a53".to_string()]));
    cpu0.add_property(Property::Reg {
        entries: vec![RegEntry::new(0x0, Some(0x1000))],
        address_cells: 1,
        size_cells: 1,
    });
    cpu0.add_property(Property::Status(fdt_raw::Status::Okay));
    fdt.root.add_child(cpu0);

    let mut cpu1 = Node::new("cpu@1");
    cpu1.add_property(Property::DeviceType("cpu".to_string()));
    cpu1.add_property(Property::Compatible(vec!["arm,cortex-a53".to_string()]));
    cpu1.add_property(Property::Reg {
        entries: vec![RegEntry::new(0x1, Some(0x1000))],
        address_cells: 1,
        size_cells: 1,
    });
    cpu1.add_property(Property::Status(fdt_raw::Status::Okay));
    fdt.root.add_child(cpu1);

    // 添加 SOC 节点
    let mut soc = Node::new("soc");
    soc.add_property(Property::Compatible(vec!["simple-bus".to_string()]));
    soc.add_property(Property::Reg {
        entries: vec![RegEntry::new(0x40000000, Some(0x100000))],
        address_cells: 1,
        size_cells: 1,
    });
    soc.add_property(Property::Ranges {
        entries: vec![],
        child_address_cells: 1,
        parent_address_cells: 1,
        size_cells: 1,
    });

    // 添加 UART 节点
    let mut uart = Node::new("uart@9000000");
    uart.add_property(Property::Compatible(vec![
        "arm,pl011".to_string(),
        "arm,primecell".to_string(),
    ]));
    uart.add_property(Property::Reg {
        entries: vec![RegEntry::new(0x9000000, Some(0x1000))],
        address_cells: 1,
        size_cells: 1,
    });
    uart.add_property(Property::Raw(fdt_edit::RawProperty::from_u32(
        "interrupts",
        0x12345678,
    )));
    uart.add_property(Property::Status(fdt_raw::Status::Okay));
    soc.add_child(uart);

    fdt.root.add_child(soc);

    // 生成 DTS
    let mut output = String::new();
    write!(&mut output, "{}", fdt).unwrap();

    // 输出结果（在实际应用中，这可以写入文件）
    // 注意：在 no_std 环境中，我们无法使用 println!

    // 创建一个测试来验证输出
    assert!(output.contains("// Device Tree Source"));
    assert!(output.contains("/dts-v1/;"));
    assert!(output.contains("/memreserve/"));
    assert!(output.contains("0x80000000 0x200000"));
    assert!(output.contains("#address-cells = <1>"));
    assert!(output.contains("#size-cells = <1>"));
    assert!(output.contains("compatible = \"vendor,board\""));
    assert!(output.contains("model = \"Demo Board\""));
    assert!(output.contains("cpu@0 {"));
    assert!(output.contains("cpu@1 {"));
    assert!(output.contains("soc {"));
    assert!(output.contains("uart@9000000 {"));

    // 示例：将输出长度记录（在实际应用中可以将此写入文件）
    let _output_length = output.len();

    // 在调试时，可以检查输出
    // 这里我们只是验证功能正常工作
}
