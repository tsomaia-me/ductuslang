use std::fmt;
use std::str::FromStr;

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Signal {
    Crash,
    Slow,
    Order,
    Rename,
    Noise,
    Scaling,
    Memory,
}

impl Signal {
    pub const ALL: [Signal; 7] = [
        Signal::Crash,
        Signal::Slow,
        Signal::Order,
        Signal::Rename,
        Signal::Noise,
        Signal::Scaling,
        Signal::Memory,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Signal::Crash => "crash",
            Signal::Slow => "slow",
            Signal::Order => "order",
            Signal::Rename => "rename",
            Signal::Noise => "noise",
            Signal::Scaling => "scaling",
            Signal::Memory => "memory",
        }
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UnknownSignal(pub String);

impl fmt::Display for UnknownSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown signal `{}`", self.0)
    }
}

impl FromStr for Signal {
    type Err = UnknownSignal;

    fn from_str(text: &str) -> Result<Signal, UnknownSignal> {
        Signal::ALL
            .into_iter()
            .find(|signal| signal.as_str() == text)
            .ok_or_else(|| UnknownSignal(text.to_string()))
    }
}
