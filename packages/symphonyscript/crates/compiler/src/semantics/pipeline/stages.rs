#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::time::{Duration, Instant};

#[cfg(test)]
thread_local! {
    static STAGES: RefCell<Vec<(&'static str, Duration)>> = const { RefCell::new(Vec::new()) };
    static NODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(crate) struct Stages {
    #[cfg(test)]
    started: Instant,
}

impl Stages {
    pub fn begin(nodes: usize) -> Self {
        #[cfg(test)]
        NODES.with(|count| count.set(nodes));
        #[cfg(not(test))]
        let _ = nodes;
        #[cfg(test)]
        STAGES.with(|stages| stages.borrow_mut().clear());
        Stages {
            #[cfg(test)]
            started: Instant::now(),
        }
    }

    pub fn mark(&mut self, name: &'static str) {
        #[cfg(test)]
        {
            let now = Instant::now();
            let spent = now - self.started;
            self.started = now;
            STAGES.with(|stages| stages.borrow_mut().push((name, spent)));
        }
        #[cfg(not(test))]
        let _ = name;
    }
}

#[cfg(test)]
pub(crate) fn last() -> Vec<(&'static str, Duration)> {
    STAGES.with(|stages| stages.borrow().clone())
}

#[cfg(test)]
pub(crate) fn nodes() -> usize {
    NODES.with(std::cell::Cell::get)
}
