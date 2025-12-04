use fdt_raw::{Phandle, Status};

use crate::{PropertyOp, RawProperty, prop::PropertyTrait};

#[derive(Clone)]
pub struct PropPhandle(pub(crate) RawProperty);

impl PropertyTrait for PropPhandle {
    fn as_raw(&self) -> &RawProperty {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawProperty {
        &mut self.0
    }
}

impl PropertyOp for PropPhandle {}

impl PropPhandle {
    pub fn new(name: &str, phandle: Phandle) -> Self {
        let data = (phandle.raw()).to_be_bytes();
        let raw = RawProperty::new(name, data.to_vec());
        Self(raw)
    }

    pub fn value(&self) -> Phandle {
        let data = self.0.data.as_slice();
        if data.len() != 4 {
            return Phandle::from(0);
        }
        Phandle::from(u32::from_be_bytes([data[0], data[1], data[2], data[3]]))
    }

    pub fn set_value(&mut self, phandle: Phandle) {
        let data = phandle.raw().to_be_bytes();
        self.0.data = data.to_vec();
    }
}

#[derive(Clone)]
pub struct PropStatus(pub(crate) RawProperty);

impl PropertyTrait for PropStatus {
    fn as_raw(&self) -> &RawProperty {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawProperty {
        &mut self.0
    }
}

impl PropertyOp for PropStatus {}
impl PropStatus {
    pub fn new(status: Status) -> Self {
        let raw = RawProperty::from_string("status", &status);
        Self(raw)
    }

    pub fn value(&self) -> Status {
        let s = self.as_string_list().pop().unwrap();

        match s.as_str() {
            "okay" => Status::Okay,
            "disabled" => Status::Disabled,
            _ => panic!("Unknown status string: {}", s),
        }
    }

    pub fn set_value(&mut self, status: Status) {
        self.0 = RawProperty::from_string("status", &status);
    }
}

impl core::fmt::Debug for PropStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "status = {}", self.value())
    }
}

impl core::fmt::Debug for PropPhandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "phandle = <{:#x}>", self.value().raw())
    }
}
