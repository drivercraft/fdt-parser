use alloc::{string::String, vec::Vec};

use super::{NodeOp, NodeTrait, RawNode};
use crate::{FdtContext, Phandle, prop::PropertyKind};

/// 时钟提供者类型
#[derive(Clone, Debug)]
pub enum ClockType {
    /// 固定时钟
    Fixed(FixedClock),
    /// 普通时钟提供者
    Normal,
}

/// 固定时钟
#[derive(Clone, Debug)]
pub struct FixedClock {
    pub name: Option<String>,
    /// 时钟频率 (Hz)
    pub frequency: u32,
    /// 时钟精度
    pub accuracy: Option<u32>,
}

/// 时钟提供者节点
#[derive(Clone, Debug)]
pub struct NodeClock {
    pub(crate) raw: RawNode,
    pub clock_names: Vec<String>,
    pub clock_output_names: Vec<String>,
    pub clock_cells: u32,
    pub kind: ClockType,
}

impl NodeOp for NodeClock {}

impl NodeTrait for NodeClock {
    fn as_raw(&self) -> &RawNode {
        &self.raw
    }

    fn as_raw_mut(&mut self) -> &mut RawNode {
        &mut self.raw
    }

    fn to_raw(self) -> RawNode {
        self.raw
    }
}

impl NodeClock {
    pub fn new(raw: RawNode) -> Self {
        let clock_output_names = Self::get_string_list(&raw, "clock-output-names");
        let clock_names = Self::get_string_list(&raw, "clock-names");
        let clock_cells = Self::get_u32(&raw, "#clock-cells").unwrap_or(0);

        let kind = if raw.compatibles().contains(&"fixed-clock") {
            let frequency = Self::get_u32(&raw, "clock-frequency").unwrap_or(0);
            let accuracy = Self::get_u32(&raw, "clock-accuracy");
            let name = clock_output_names.first().cloned();

            ClockType::Fixed(FixedClock {
                name,
                frequency,
                accuracy,
            })
        } else {
            ClockType::Normal
        };

        NodeClock {
            clock_output_names,
            clock_names,
            clock_cells,
            kind,
            raw,
        }
    }

    /// 获取字符串列表属性
    fn get_string_list(raw: &RawNode, name: &str) -> Vec<String> {
        let Some(prop) = raw.find_property(name) else {
            return Vec::new();
        };
        match &prop.kind {
            PropertyKind::StringList(v) => v.clone(),
            PropertyKind::Str(s) => vec![s.clone()],
            _ => Vec::new(),
        }
    }

    /// 获取 u32 属性
    fn get_u32(raw: &RawNode, name: &str) -> Option<u32> {
        let prop = raw.find_property(name)?;
        match &prop.kind {
            PropertyKind::Num(v) => Some(*v as u32),
            _ => None,
        }
    }

    /// 获取时钟输出名称（用于 provider）
    pub fn output_name(&self, index: usize) -> Option<&str> {
        self.clock_output_names.get(index).map(|s| s.as_str())
    }

    /// 获取时钟名称（用于 consumer）
    pub fn clock_name(&self, index: usize) -> Option<&str> {
        self.clock_names.get(index).map(|s| s.as_str())
    }

    /// 使用 FdtContext 解析 clocks 属性
    ///
    /// 通过查找每个 phandle 对应的 clock provider 的 #clock-cells，
    /// 正确解析 specifier 的长度。
    pub fn clocks_with_context<'a>(&self, ctx: &FdtContext<'a>) -> Vec<ClockRef> {
        let Some(prop) = self.raw.find_property("clocks") else {
            return Vec::new();
        };

        let PropertyKind::Raw(raw_prop) = &prop.kind else {
            return Vec::new();
        };

        let data = raw_prop.data();
        if data.len() < 4 {
            return Vec::new();
        }

        let mut clocks = Vec::new();
        let mut offset = 0;
        let mut index = 0;

        while offset + 4 <= data.len() {
            // 读取 phandle
            let phandle_val = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            let phandle = Phandle::from(phandle_val);
            offset += 4;

            // 通过 phandle 查找 provider 节点，获取其 #clock-cells
            let clock_cells = ctx
                .find_by_phandle(phandle)
                .and_then(|node| {
                    node.find_property("#clock-cells")
                        .and_then(|p| match &p.kind {
                            PropertyKind::Num(v) => Some(*v as usize),
                            _ => None,
                        })
                })
                .unwrap_or(1); // 默认 1 cell

            // 读取 specifier（根据 provider 的 #clock-cells）
            let mut specifier = Vec::with_capacity(clock_cells);
            let mut complete = true;
            for _ in 0..clock_cells {
                if offset + 4 <= data.len() {
                    let val = u32::from_be_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]);
                    specifier.push(val);
                    offset += 4;
                } else {
                    // 数据不足，停止解析
                    complete = false;
                    break;
                }
            }

            // 只有完整的 clock reference 才添加
            if !complete {
                break;
            }

            // 从 clock-names 获取对应的名称
            let name = self.clock_names.get(index).cloned();

            clocks.push(ClockRef::with_name(
                name,
                phandle,
                clock_cells as u32,
                specifier,
            ));
            index += 1;
        }

        clocks
    }
}

/// 时钟引用，用于解析 clocks 属性
///
/// 根据设备树规范，clocks 属性格式为：
/// `clocks = <&clock_provider specifier [specifier ...]> [<&clock_provider2 ...>]`
///
/// 每个时钟引用由一个 phandle 和若干个 specifier cells 组成，
/// specifier 的数量由目标 clock provider 的 `#clock-cells` 属性决定。
#[derive(Clone, Debug)]
pub struct ClockRef {
    /// 时钟的名称，来自 clock-names 属性
    pub name: Option<String>,
    /// 时钟提供者的 phandle
    pub phandle: Phandle,
    /// provider 的 #clock-cells 值
    pub cells: u32,
    /// 时钟选择器（specifier），通常第一个值用于选择时钟输出
    /// 长度由 provider 的 #clock-cells 决定
    pub specifier: Vec<u32>,
}

impl ClockRef {
    /// 创建一个新的时钟引用
    pub fn new(phandle: Phandle, cells: u32, specifier: Vec<u32>) -> Self {
        Self {
            name: None,
            phandle,
            cells,
            specifier,
        }
    }

    /// 创建一个带名称的时钟引用
    pub fn with_name(
        name: Option<String>,
        phandle: Phandle,
        cells: u32,
        specifier: Vec<u32>,
    ) -> Self {
        Self {
            name,
            phandle,
            cells,
            specifier,
        }
    }

    /// 获取选择器的第一个值（通常用于选择时钟输出）
    ///
    /// 只有当 `cells > 0` 时才返回选择器值，
    /// 因为 `#clock-cells = 0` 的 provider 不需要选择器。
    pub fn select(&self) -> Option<u32> {
        if self.cells > 0 {
            self.specifier.first().copied()
        } else {
            None
        }
    }
}

/// 带上下文的 NodeClock 引用
///
/// 可以直接调用 `clocks()` 方法解析时钟引用
pub struct NodeClockRef<'a> {
    pub clock: &'a NodeClock,
    pub ctx: &'a FdtContext<'a>,
}

impl<'a> NodeClockRef<'a> {
    /// 创建新的带上下文的 NodeClock 引用
    pub fn new(clock: &'a NodeClock, ctx: &'a FdtContext<'a>) -> Self {
        Self { clock, ctx }
    }

    /// 解析 clocks 属性，返回时钟引用列表
    pub fn clocks(&self) -> Vec<ClockRef> {
        self.clock.clocks_with_context(self.ctx)
    }

    /// 获取 #clock-cells
    pub fn clock_cells(&self) -> u32 {
        self.clock.clock_cells
    }

    /// 获取时钟类型
    pub fn kind(&self) -> &ClockType {
        &self.clock.kind
    }

    /// 获取节点名称
    pub fn name(&self) -> &str {
        self.clock.name()
    }

    /// 获取时钟输出名称列表
    pub fn clock_output_names(&self) -> &[String] {
        &self.clock.clock_output_names
    }

    /// 获取时钟名称列表
    pub fn clock_names(&self) -> &[String] {
        &self.clock.clock_names
    }
}
