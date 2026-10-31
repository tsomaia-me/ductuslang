use symphonyscript_core::FamilyId;

use super::arg_key::ArgKey;
use super::arg_shape::ArgShape;
use super::arg_sig::ArgSig;
use super::c_val::CVal;

#[derive(Default)]
pub(crate) struct CallScratch {
    pub keys: Vec<ArgKey>,
    pub shapes: Vec<ArgShape>,
    pub values: Vec<CVal>,
    pub leaves: Vec<(ArgKey, ArgSig)>,
    pub units: Vec<(FamilyId, usize)>,
}

impl CallScratch {
    pub fn clear(&mut self) {
        self.keys.clear();
        self.shapes.clear();
        self.values.clear();
        self.leaves.clear();
        self.units.clear();
    }
}
