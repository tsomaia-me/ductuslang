use alloc::string::String;
use alloc::vec::Vec;

use crate::types::Value;

pub struct NativeOut<'a> {
    slots: Slots<'a>,
    len: usize,
    text: Option<&'a mut String>,
}

enum Slots<'a> {
    Fixed(&'a mut [Value]),
    Growing(&'a mut Vec<Value>, usize),
}

impl<'a> NativeOut<'a> {
    pub fn new(elements: &'a mut [Value], text: Option<&'a mut String>) -> Self {
        Self {
            slots: Slots::Fixed(elements),
            len: 0,
            text,
        }
    }

    pub fn growing(
        elements: &'a mut Vec<Value>,
        capacity: usize,
        text: Option<&'a mut String>,
    ) -> Self {
        elements.clear();
        Self {
            slots: Slots::Growing(elements, capacity),
            len: 0,
            text,
        }
    }

    pub fn set(&mut self, value: Value) -> bool {
        if self.write(0, value) {
            self.len = 1;
            true
        } else {
            false
        }
    }

    pub fn push(&mut self, value: Value) -> bool {
        if self.write(self.len, value) {
            self.len += 1;
            true
        } else {
            false
        }
    }

    pub fn set_text(&mut self, text: &str) -> bool {
        match self.text.as_deref_mut() {
            Some(out) => {
                out.clear();
                out.push_str(text);
                true
            }
            None => false,
        }
    }

    pub fn capacity(&self) -> usize {
        match &self.slots {
            Slots::Fixed(elements) => elements.len(),
            Slots::Growing(_, capacity) => *capacity,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn write(&mut self, position: usize, value: Value) -> bool {
        match &mut self.slots {
            Slots::Fixed(elements) => match elements.get_mut(position) {
                Some(slot) => {
                    *slot = value;
                    true
                }
                None => false,
            },
            Slots::Growing(elements, capacity) => {
                if position >= *capacity {
                    return false;
                }
                match elements.get_mut(position) {
                    Some(slot) => *slot = value,
                    None => elements.push(value),
                }
                true
            }
        }
    }
}
