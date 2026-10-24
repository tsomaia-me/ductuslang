use alloc::string::String;
use alloc::vec::Vec;

use super::array_element::ArrayElement;
use super::attr_default::AttrDefault;
use super::sealed::Sealed;
use crate::types::Ty;

pub trait AttrType: Sealed {
    fn ty() -> Ty;
    fn into_default(self) -> AttrDefault;
}

impl<T: ArrayElement> AttrType for T {
    fn ty() -> Ty {
        T::element_ty()
    }

    fn into_default(self) -> AttrDefault {
        let mut elements = Vec::new();
        self.push_elements(&mut elements);
        AttrDefault::Elements(elements)
    }
}

impl AttrType for &'static str {
    fn ty() -> Ty {
        Ty::STR
    }

    fn into_default(self) -> AttrDefault {
        AttrDefault::Text(String::from(self))
    }
}
