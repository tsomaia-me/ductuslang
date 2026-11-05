use serde::Serialize;
use symphonyscript::Selector;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    New,
    Old,
}

impl Engine {
    pub fn of(selector: Selector) -> Engine {
        if selector == Selector::NewCore {
            Engine::New
        } else {
            Engine::Old
        }
    }
}
