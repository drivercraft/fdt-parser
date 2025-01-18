use fdt_parser::Fdt;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let bytes = include_bytes!("../../dtb/bcm2711-rpi-4-b.dtb");

    let fdt = Fdt::from_bytes(bytes).unwrap();
    println!("version: {}", fdt.version());
    for region in fdt.memory_reservation_block() {
        println!("region: {:?}", region);
    }
    for (i, node) in fdt.all_nodes().enumerate() {
        if i > 40 {
            break;
        }
        let space = " ".repeat((node.level - 1) * 4);
        println!("{}{}", space, node.name());

        if let Some(cap) = node.compatible() {
            println!("{} -compatible: ", space);
            for cap in cap {
                println!("{}     {:?}", space, cap.unwrap());
            }
        }

        if let Some(reg) = node.reg() {
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
