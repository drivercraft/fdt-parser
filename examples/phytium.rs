use fdt_parser::Fdt;

fn main() {
    let bytes = include_bytes!("../dtb/phytium.dtb");
    let fdt = Fdt::from_bytes(bytes).unwrap();
    // println!("version: {}", fdt.version());
    // for region in fdt.reserved_memory_regions() {
    //     println!("region: {:?}", region);
    // }

    // for node in fdt.all_nodes() {
    //     let space = " ".repeat(node.level * 4);
    //     println!("{}{}", space, node.name());

    //     if node.reg().count() > 0 {
    //         println!("{} - reg: ", space);
    //         for cell in node.reg() {
    //             let s = cell
    //                 .into_iter()
    //                 .map(|o| format!("{:x}", o))
    //                 .collect::<Vec<_>>()
    //                 .join(", ");
    //             println!("{}     <{}>", space, s);
    //         }
    //     }

    //     for prop in node.propertys() {
    //         println!("{} - {}", space, prop.name);
    //     }
    // }
}
