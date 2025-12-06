use alloc::{string::String, vec::Vec};

use super::{NodeOp, NodeTrait, RawNode};
use crate::{Phandle, prop::PropertyKind};

/// 时钟信息
#[derive(Clone, Debug)]
pub struct ClockInfo {
    /// 消费者通过 `clock-names` 提供的名称
    pub name: Option<String>,
    /// 提供者通过 `clock-output-names` 暴露的名称
    pub provider_output_name: Option<String>,
    /// 时钟提供者的 phandle
    pub phandle: Phandle,
    /// 时钟选择器索引
    pub select: u64,
}

/// 时钟提供者类型
#[derive(Clone, Debug)]
pub enum ClockType {
    /// 固定时钟
    Fixed(FixedClock),
    /// 普通时钟提供者
    Provider,
}

/// 固定时钟
#[derive(Clone, Debug)]
pub struct FixedClock {
    /// 时钟频率 (Hz)
    pub frequency: u32,
    /// 时钟精度
    pub accuracy: Option<u32>,
}

/// 时钟提供者节点
#[derive(Clone, Debug)]
pub struct NodeClock {
    pub(crate) raw: RawNode,
    pub output_names: Vec<String>,
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
        let output_names = Self::get_output_names(&raw).to_vec();
        let PropertyKind::Num(cells) = raw
            .find_property("#clock-cells")
            .map(|p| &p.kind)
            .unwrap_or(&PropertyKind::Num(0))
        else {
            panic!("#clock-cells property is not Num");
        };
        let cells = *cells as u32;
        let kind = if raw.compatibles().contains(&"fixed-clock") {
            let PropertyKind::Num(freq) = raw
                .find_property("clock-frequency")
                .map(|p| &p.kind)
                .unwrap_or(&PropertyKind::Num(0))
            else {
                panic!("clock-frequency property is not Num");
            };

            let acc = if let Some(prop) = raw.find_property("clock-accuracy") {
                match &prop.kind {
                    PropertyKind::Num(v) => Some(*v as u32),
                    _ => panic!("clock-accuracy property is not Num"),
                }
            } else {
                None
            };

            ClockType::Fixed(FixedClock {
                frequency: *freq as u32,
                accuracy: acc,
            })
        } else {
            ClockType::Provider
        };

        NodeClock {
            clock_cells: cells,
            kind,
            output_names,
            raw,
        }
    }

    fn get_output_names(raw: &RawNode) -> &[String] {
        let Some(prop) = raw.find_property("clock-output-names") else {
            return &[];
        };

        match &prop.kind {
            PropertyKind::StringList(v) => v,
            _ => panic!("clock-output-names property is not StringList"),
        }
    }
}

/// 时钟引用，用于解析 clocks 属性
#[derive(Clone, Debug)]
pub struct ClockRef {
    /// 时钟提供者的 phandle
    pub phandle: Phandle,
    /// 时钟选择器
    pub select: u64,
}

impl ClockRef {
    pub fn new(phandle: Phandle, select: u64) -> Self {
        Self { phandle, select }
    }
}
