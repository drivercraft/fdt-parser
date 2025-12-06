#![no_std]

#[macro_use]
extern crate alloc;

mod ctx;
mod display;
mod encode;
mod fdt;
mod node;
mod prop;

pub use ctx::FdtContext;
pub use display::FmtLevel;
pub use encode::{FdtData, FdtEncoder, NodeEncode};
pub use fdt::{Fdt, MemoryReservation};
pub use node::{
    ClockInfo, ClockRef, ClockType, FixedClock, MemoryRegion, Node, NodeChosen, NodeClock,
    NodeInterruptController, NodeMemory, NodeMut, NodeOp, NodePci, NodeRef, PciRange, PciSpace,
};
pub use prop::{Phandle, Property, RangesEntry, RawProperty, RegInfo, Status};
