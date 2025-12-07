use core::fmt;

use alloc::vec::Vec;

use crate::{
    ClockType, Node, NodeKind, NodeMut, NodeRef, NodeRefClock, NodeRefInterruptController, NodeRefMemory,
    Property,
};

/// Node 的 DTS 显示格式化器
pub struct NodeDisplay<'a> {
    node: &'a Node,
    indent: usize,
    show_address: bool,
    show_size: bool,
}

impl<'a> NodeDisplay<'a> {
    pub fn new(node: &'a Node) -> Self {
        Self {
            node,
            indent: 0,
            show_address: true,
            show_size: true,
        }
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn show_address(mut self, show: bool) -> Self {
        self.show_address = show;
        self
    }

    pub fn show_size(mut self, show: bool) -> Self {
        self.show_size = show;
        self
    }

    fn format_indent(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.indent {
            write!(f, "    ")?;
        }
        Ok(())
    }

    fn format_node_name(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.node.name)?;

        // 如果节点名为空（根节点），输出为 /
        if self.node.name.is_empty() {
            write!(f, "/")?;
        }
        Ok(())
    }

    fn format_property(&self, prop: &Property, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format_indent(f)?;
        match prop.name() {
            "reg" => {
                if self.show_address || self.show_size {
                    write!(f, "reg = ")?;
                    self.format_reg_values(prop, f)?;
                } else {
                    write!(f, "reg;")?;
                }
            }
            "compatible" => {
                write!(f, "compatible = ")?;
                self.format_string_list(prop, f)?;
            }
            "clock-names" | "pinctrl-names" | "reg-names" => {
                write!(f, "{} = ", prop.name())?;
                self.format_string_list(prop, f)?;
            }
            "interrupt-controller" | "#address-cells" | "#size-cells" | "#interrupt-cells"
            | "#clock-cells" | "phandle" => {
                write!(f, "{};", prop.name())?;
            }
            _ => {
                write!(f, "{} = ", prop.name())?;
                self.format_property_value(prop, f)?;
            }
        }
        writeln!(f)
    }

    fn format_reg_values(&self, prop: &Property, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut reader = prop.as_reader();
        let mut first = true;
        write!(f, "<")?;

        // 获取 parent 的 address-cells 和 size-cells
        // 这里需要从上下文获取，暂时使用默认值
        let address_cells = 2; // 默认值
        let size_cells = 1;     // 默认值

        while let (Some(addr), Some(size)) = (
            reader.read_cells(address_cells),
            reader.read_cells(size_cells),
        ) {
            if !first {
                write!(f, " ")?;
            }
            first = false;

            if self.show_address {
                write!(f, "0x{:x}", addr)?;
            }
            if self.show_size && size > 0 {
                if self.show_address {
                    write!(f, " ")?;
                }
                write!(f, "0x{:x}", size)?;
            }
        }

        write!(f, ">;")
    }

    fn format_string_list(&self, prop: &Property, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let iter = prop.as_str_iter();
        let mut first = true;
        write!(f, "\"")?;
        for s in iter {
            if !first {
                write!(f, "\", \"")?;
            }
            first = false;
            write!(f, "{}", s)?;
        }
        write!(f, "\";")
    }

    fn format_property_value(&self, prop: &Property, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = prop.as_str() {
            write!(f, "\"{}\";", s)
        } else if let Some(u32_val) = prop.get_u32() {
            write!(f, "<0x{:x}>;", u32_val)
        } else if let Some(u64_val) = prop.get_u64() {
            write!(f, "<0x{:x}>;", u64_val)
        } else {
            // 尝试格式化为字节数组
            let mut reader = prop.as_reader();
            let mut first = true;
            write!(f, "<")?;
            while let Some(val) = reader.read_u32() {
                if !first {
                    write!(f, " ")?;
                }
                first = false;
                write!(f, "0x{:02x}", val)?;
            }
            write!(f, ">;")
        }
    }
}

impl<'a> fmt::Display for NodeDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format_indent(f)?;

        if self.node.name.is_empty() {
            // 根节点
            writeln!(f, "/ {{")?;
        } else {
            // 普通节点
            write!(f, "{}", self.node.name)?;

            // 检查是否有地址和大小属性需要显示
            let mut props = Vec::new();
            for prop in self.node.properties() {
                if prop.name() != "reg" {
                    props.push(prop);
                }
            }

            if !props.is_empty() {
                writeln!(f, " {{")?;
            } else {
                writeln!(f, ";")?;
                return Ok(());
            }
        }

        // 输出属性
        for prop in self.node.properties() {
            if prop.name() != "reg" || self.show_address || self.show_size {
                self.format_property(prop, f)?;
            }
        }

        // 输出子节点
        for child in self.node.children() {
            let child_display = NodeDisplay::new(child)
                .indent(self.indent + 1)
                .show_address(self.show_address)
                .show_size(self.show_size);
            write!(f, "{}", child_display)?;
        }

        // 关闭节点
        if !self.node.name.is_empty() || !self.node.children.is_empty() {
            self.format_indent(f)?;
            writeln!(f, "}};")?;
        } else {
            self.format_indent(f)?;
            writeln!(f, "}};")?;
        }

        Ok(())
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display = NodeDisplay::new(self);
        write!(f, "{}", display)
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("name", &self.name)
            .field("children_count", &self.children.len())
            .field("properties_count", &self.properties.len())
            .field("phandle", &self.phandle())
            .field("address_cells", &self.address_cells())
            .field("size_cells", &self.size_cells())
            .finish()
    }
}

/// NodeRef 的显示格式化器
pub struct NodeRefDisplay<'a> {
    node_ref: &'a NodeRef<'a>,
    indent: usize,
    show_details: bool,
}

impl<'a> NodeRefDisplay<'a> {
    pub fn new(node_ref: &'a NodeRef<'a>) -> Self {
        Self {
            node_ref,
            indent: 0,
            show_details: true,
        }
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn show_details(mut self, show: bool) -> Self {
        self.show_details = show;
        self
    }

    fn format_type_info(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.node_ref.as_ref() {
            NodeKind::Clock(clock) => {
                write!(f, "Clock Node: ")?;
                if let ClockType::Fixed(fixed) = &clock.kind {
                    write!(f, "Fixed Clock (freq={}Hz", fixed.frequency)?;
                    if let Some(accuracy) = fixed.accuracy {
                        write!(f, ", accuracy={})", accuracy)?;
                    }
                    write!(f, ")")?;
                } else {
                    write!(f, "Clock Provider")?;
                }
                if !clock.clock_output_names.is_empty() {
                    write!(f, ", outputs: {:?}", clock.clock_output_names)?;
                }
                write!(f, ", cells={}", clock.clock_cells)?;
            }
            NodeKind::Pci(pci) => {
                write!(f, "PCI Node")?;
                if let Some(bus_range) = pci.bus_range() {
                    write!(f, " (bus range: {:?})", bus_range)?;
                }
                write!(f, ", interrupt-cells={}", pci.interrupt_cells())?;
            }
            NodeKind::InterruptController(ic) => {
                write!(f, "Interrupt Controller")?;
                if let Some(cells) = ic.interrupt_cells() {
                    write!(f, " (interrupt-cells={})", cells)?;
                }
                let compatibles = ic.compatibles();
                if !compatibles.is_empty() {
                    write!(f, ", compatible: {:?}", compatibles)?;
                }
            }
            NodeKind::Memory(mem) => {
                write!(f, "Memory Node")?;
                let regions = mem.regions();
                if !regions.is_empty() {
                    write!(f, " ({} regions)", regions.len())?;
                    for (i, region) in regions.iter().take(3).enumerate() {
                        write!(f, "\n    [{}]: 0x{:x}-0x{:x}",
                               i, region.address, region.address + region.size)?;
                    }
                }
                if let Some(dt) = mem.device_type() {
                    write!(f, ", device_type={}", dt)?;
                }
            }
            NodeKind::Generic(_) => {
                write!(f, "Generic Node")?;
            }
        }
        Ok(())
    }
}

impl<'a> fmt::Display for NodeRefDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.indent {
            write!(f, "    ")?;
        }

        if self.show_details {
            write!(f, "{}: ", self.node_ref.name())?;
            self.format_type_info(f)?;
            write!(f, "\n")?;

            // 添加缩进并显示 DTS
            let dts_display = NodeDisplay::new(self.node_ref)
                .indent(self.indent + 1);
            write!(f, "{}", dts_display)?;
        } else {
            write!(f, "{}", self.node_ref.name())?;
        }

        Ok(())
    }
}

impl fmt::Display for NodeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display = NodeRefDisplay::new(self);
        write!(f, "{}", display)
    }
}

impl fmt::Debug for NodeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeRef")
            .field("name", &self.name())
            .field("path", &self.path())
            .field("node_type", &match self.as_ref() {
                NodeKind::Clock(_) => "Clock",
                NodeKind::Pci(_) => "PCI",
                NodeKind::InterruptController(_) => "InterruptController",
                NodeKind::Memory(_) => "Memory",
                NodeKind::Generic(_) => "Generic",
            })
            .field("phandle", &self.phandle())
            .finish()
    }
}

impl fmt::Debug for NodeRefClock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeRefClock")
            .field("name", &self.name())
            .field("clock_cells", &self.clock_cells)
            .field("clock_type", &self.kind)
            .field("output_names", &self.clock_output_names)
            .field("phandle", &self.phandle())
            .finish()
    }
}

impl fmt::Debug for NodeRefInterruptController<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeRefInterruptController")
            .field("name", &self.name())
            .field("interrupt_cells", &self.interrupt_cells())
            .field("interrupt_address_cells", &self.interrupt_address_cells())
            .field("is_interrupt_controller", &self.is_interrupt_controller())
            .field("compatibles", &self.compatibles())
            .field("phandle", &self.phandle())
            .finish()
    }
}

impl fmt::Debug for NodeRefMemory<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeRefMemory")
            .field("name", &self.name())
            .field("regions_count", &self.regions().len())
            .field("device_type", &self.device_type())
            .field("phandle", &self.phandle())
            .finish()
    }
}

impl fmt::Display for NodeRefClock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node_ref = crate::NodeRef::Clock(self.clone());
        let display = NodeRefDisplay::new(&node_ref);
        write!(f, "{}", display)
    }
}

impl fmt::Display for NodeRefInterruptController<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node_ref = crate::NodeRef::InterruptController(self.clone());
        let display = NodeRefDisplay::new(&node_ref);
        write!(f, "{}", display)
    }
}

impl fmt::Display for NodeRefMemory<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node_ref = crate::NodeRef::Memory(self.clone());
        let display = NodeRefDisplay::new(&node_ref);
        write!(f, "{}", display)
    }
}

impl fmt::Display for NodeMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeMut::Gerneric(generic) => {
                let display = NodeDisplay::new(&generic.node);
                write!(f, "{}", display)
            }
        }
    }
}

impl fmt::Debug for NodeMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeMut")
            .field("name", &match self {
                NodeMut::Gerneric(generic) => generic.node.name(),
            })
            .field("node_type", &"Generic")
            .field("children_count", &match self {
                NodeMut::Gerneric(generic) => generic.node.children.len(),
            })
            .field("properties_count", &match self {
                NodeMut::Gerneric(generic) => generic.node.properties.len(),
            })
            .finish()
    }
}