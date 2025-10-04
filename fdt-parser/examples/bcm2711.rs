use fdt_parser::Fdt;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let bytes = include_bytes!("../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");

    let fdt = Fdt::from_bytes(bytes).unwrap();
    println!("version: {}", fdt.version());
    for region in fdt.memory_reservaion_blocks() {
        println!("region: {:?}", region);
    }
    for (i, node) in fdt.all_nodes().into_iter().enumerate() {
        if i > 40 {
            break;
        }
        let space = " ".repeat(node.level().saturating_sub(1) * 4);
        println!("{}{}", space, node.name());

        let compatibles = node.compatibles();
        if !compatibles.is_empty() {
            println!("{} -compatible: ", space);
            for cap in compatibles {
                println!("{}     {:?}", space, cap);
            }
        }

        if let Ok(reg) = node.reg() {
            println!("{} - reg: ", space);
            for cell in reg {
                println!("{}     {:?}", space, cell);
            }
        }

        if let Some(status) = node.status() {
            println!("{} - status: {:?}", space, status);
        }
    }
}
