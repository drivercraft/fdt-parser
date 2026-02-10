use dtb_file::*;
use fdt_edit::*;

#[test]
fn test_iter_nodes() {
    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();
    let mut count = 0;
    for view in fdt.all_nodes() {
        println!("{:?} path={}", view.as_node(), view.path());
        count += 1;
    }
    assert!(count > 0, "should have at least one node");
    assert_eq!(count, fdt.node_count());
}

#[test]
fn test_node_classify() {
    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();

    let mut memory_count = 0;
    let mut intc_count = 0;
    let mut generic_count = 0;

    for view in fdt.all_nodes() {
        match view {
            NodeType::Memory(mem) => {
                memory_count += 1;
                let regions = mem.regions();
                println!(
                    "Memory node: {} regions={} total_size={:#x}",
                    mem.path(),
                    regions.len(),
                    mem.total_size()
                );
            }
            NodeType::InterruptController(intc) => {
                intc_count += 1;
                println!(
                    "IntC node: {} #interrupt-cells={:?}",
                    intc.path(),
                    intc.interrupt_cells()
                );
            }
            NodeType::Generic(g) => {
                generic_count += 1;
                let _ = g.path();
            }
        }
    }

    println!(
        "memory={}, intc={}, generic={}",
        memory_count, intc_count, generic_count
    );
    assert!(memory_count > 0, "phytium DTB should have memory nodes");
    assert!(intc_count > 0, "phytium DTB should have intc nodes");
    assert!(generic_count > 0, "phytium DTB should have generic nodes");
}

#[test]
fn test_visit_trait() {
    struct Counter {
        nodes: usize,
        memory: usize,
        intc: usize,
    }

    impl<'a> Visit<'a> for Counter {
        fn visit_memory_node(&mut self, node: NodeView<'a>) {
            self.memory += 1;
            self.nodes += 1;
            visit_node_children(self, node);
        }

        fn visit_intc_node(&mut self, node: NodeView<'a>) {
            self.intc += 1;
            self.nodes += 1;
            visit_node_children(self, node);
        }

        fn visit_generic_node(&mut self, node: NodeView<'a>) {
            self.nodes += 1;
            visit_node_children(self, node);
        }
    }

    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();

    let mut counter = Counter {
        nodes: 0,
        memory: 0,
        intc: 0,
    };
    fdt.visit(&mut counter);

    println!(
        "Visit: total={} memory={} intc={}",
        counter.nodes, counter.memory, counter.intc
    );
    assert_eq!(counter.nodes, fdt.node_count());
}

#[test]
fn test_path_lookup() {
    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();

    // Root should always be found
    let root = fdt.get_by_path("/").unwrap();
    assert_eq!(root.as_view().id(), fdt.root_id());

    // Check path round-trip: for every node, path_of(id) should resolve back
    for id in fdt.iter_node_ids() {
        let path = fdt.path_of(id);
        let found = fdt.get_by_path_id(&path);
        assert_eq!(
            found,
            Some(id),
            "path_of({}) = {:?} did not resolve back",
            id,
            path
        );
    }

    // Verify get_by_path returns correct NodeType classification
    for view in fdt.all_nodes() {
        let path = view.path();
        let typed = fdt.get_by_path(&path).unwrap();
        assert_eq!(typed.as_view().id(), view.id());
    }
}

#[test]
fn test_display_nodes() {
    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();
    for view in fdt.all_nodes() {
        println!("{}", view);
    }
}
