use alloc::string::String;

use crate::types::Value;

pub struct NativeOut<'a> {
    elements: &'a mut [Value],
    len: usize,
    text: Option<&'a mut String>,
}

impl<'a> NativeOut<'a> {
    pub fn new(elements: &'a mut [Value], text: Option<&'a mut String>) -> Self {
        Self {
            elements,
            len: 0,
            text,
        }
    }

    pub fn set(&mut self, value: Value) -> bool {
        match self.elements.first_mut() {
            Some(slot) => {
                *slot = value;
                self.len = 1;
                true
            }
            None => false,
        }
    }

    pub fn push(&mut self, value: Value) -> bool {
        match self.elements.get_mut(self.len) {
            Some(slot) => {
                *slot = value;
                self.len += 1;
                true
            }
            None => false,
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
        self.elements.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
