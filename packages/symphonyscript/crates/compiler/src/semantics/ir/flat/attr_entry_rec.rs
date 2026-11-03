use super::attr_form::AttrForm;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct AttrEntryRec {
    pub name: u32,
    pub form: AttrForm,
    pub lo: u32,
    pub hi: u32,
}
