use core::ops::Deref;

use alloc::{string::{String, ToString}, vec::Vec};
use fdt_raw::Phandle;

use crate::node::gerneric::NodeRefGen;

/// 时钟提供者类型
#[derive(Clone, Debug, PartialEq)]
pub enum ClockType {
    /// 固定时钟
    Fixed(FixedClock),
    /// 普通时钟提供者
    Normal,
}

/// 固定时钟
#[derive(Clone, Debug, PartialEq)]
pub struct FixedClock {
    pub name: Option<String>,
    /// 时钟频率 (Hz)
    pub frequency: u32,
    /// 时钟精度
    pub accuracy: Option<u32>,
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

/// 时钟提供者节点引用
#[derive(Clone, Debug)]
pub struct NodeRefClock<'a> {
    pub node: NodeRefGen<'a>,
    pub clock_output_names: Vec<String>,
    pub clock_cells: u32,
    pub kind: ClockType,
}

impl<'a> NodeRefClock<'a> {
    pub fn try_from(node: NodeRefGen<'a>) -> Result<Self, NodeRefGen<'a>> {
        // 检查是否有时钟提供者属性
        if node.find_property("#clock-cells").is_none() {
            return Err(node);
        }

        // 获取 clock-output-names 属性
        let clock_output_names = if let Some(prop) = node.find_property("clock-output-names") {
            let iter = prop.as_str_iter();
            iter.map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        // 获取 #clock-cells
        let clock_cells = node
            .find_property("#clock-cells")
            .and_then(|prop| prop.get_u32())
            .unwrap_or(0);

        // 判断时钟类型
        let kind = if node.compatibles().any(|c| c == "fixed-clock") {
            let frequency = node
                .find_property("clock-frequency")
                .and_then(|prop| prop.get_u32())
                .unwrap_or(0);
            let accuracy = node
                .find_property("clock-accuracy")
                .and_then(|prop| prop.get_u32());
            let name = clock_output_names.first().cloned();

            ClockType::Fixed(FixedClock {
                name,
                frequency,
                accuracy,
            })
        } else {
            ClockType::Normal
        };

        Ok(Self {
            node,
            clock_output_names,
            clock_cells,
            kind,
        })
    }

    /// 获取时钟输出名称（用于 provider）
    pub fn output_name(&self, index: usize) -> Option<&str> {
        self.clock_output_names.get(index).map(|s| s.as_str())
    }

    /// 解析 clocks 属性，返回时钟引用列表
    ///
    /// 通过查找每个 phandle 对应的 clock provider 的 #clock-cells，
    /// 正确解析 specifier 的长度。
    pub fn clocks(&self) -> Vec<ClockRef> {
        let Some(prop) = self.find_property("clocks") else {
            return Vec::new();
        };

        let mut clocks = Vec::new();
        let mut data = prop.as_reader();
        let mut index = 0;

        // 获取 clock-names 用于命名
        let clock_names = if let Some(prop) = self.find_property("clock-names") {
            let iter = prop.as_str_iter();
            iter.map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        while let Some(phandle_raw) = data.read_u32() {
            let phandle = Phandle::from(phandle_raw);

            // 通过 phandle 查找 provider 节点，获取其 #clock-cells
            let clock_cells = if let Some(provider) = self.ctx.find_by_phandle(phandle) {
                provider
                    .get_property("#clock-cells")
                    .and_then(|p| p.get_u32())
                    .unwrap_or(1) // 默认 1 cell
            } else {
                1 // 默认 1 cell
            };

            // 读取 specifier（根据 provider 的 #clock-cells）
            let mut specifier = Vec::with_capacity(clock_cells as usize);
            let mut complete = true;
            for _ in 0..clock_cells {
                if let Some(val) = data.read_u32() {
                    specifier.push(val);
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
            let name = clock_names.get(index).cloned();

            clocks.push(ClockRef::with_name(
                name,
                phandle,
                clock_cells,
                specifier,
            ));
            index += 1;
        }

        clocks
    }
}

impl<'a> Deref for NodeRefClock<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}