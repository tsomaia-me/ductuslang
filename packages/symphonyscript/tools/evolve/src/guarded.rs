use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const STACK_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Guarded<T> {
    Finished { value: T, elapsed: Duration },
    Panicked { message: String },
    TimedOut,
}

pub fn guarded<T, F>(budget: Duration, work: F) -> Guarded<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("evolve-compile".to_string())
        .stack_size(STACK_BYTES)
        .spawn(move || {
            let started = Instant::now();
            let result = catch_unwind(AssertUnwindSafe(work));
            let elapsed = started.elapsed();
            let _ = sender.send((result, elapsed));
        });
    if let Err(error) = spawned {
        return Guarded::Panicked {
            message: format!("could not start the compile thread: {error}"),
        };
    }
    match receiver.recv_timeout(budget) {
        Ok((Ok(value), elapsed)) if elapsed <= budget => Guarded::Finished { value, elapsed },
        Ok((Ok(_), _)) => Guarded::TimedOut,
        Ok((Err(payload), _)) => Guarded::Panicked {
            message: panic_message(payload.as_ref()),
        },
        Err(_) => Guarded::TimedOut,
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_string();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    "panic with a non-text payload".to_string()
}
