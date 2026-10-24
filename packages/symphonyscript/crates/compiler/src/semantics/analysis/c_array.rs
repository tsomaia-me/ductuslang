use std::fmt;
use std::rc::Rc;

use super::c_val::CVal;

#[derive(Clone)]
pub(crate) struct CArray(Rc<Vec<CVal>>);

impl CArray {
    pub fn new(items: Vec<CVal>) -> CArray {
        CArray(Rc::new(items))
    }

    pub fn items(&self) -> &[CVal] {
        &self.0
    }

    pub fn lengths(&self) -> Vec<u32> {
        let mut outer_first = Vec::new();
        let mut current = Some(self);
        while let Some(array) = current {
            outer_first.push(u32::try_from(array.0.len()).unwrap_or(u32::MAX));
            current = match array.0.first() {
                Some(CVal::Array(inner)) => Some(inner),
                _ => None,
            };
        }
        outer_first.reverse();
        outer_first
    }

    pub fn leaves(&self) -> Vec<CVal> {
        let mut found = Vec::new();
        let mut stack: Vec<(&CArray, usize)> = vec![(self, 0)];
        while let Some((array, index)) = stack.pop() {
            let Some(item) = array.0.get(index) else {
                continue;
            };
            stack.push((array, index + 1));
            match item {
                CVal::Array(inner) => stack.push((inner, 0)),
                leaf => found.push(leaf.clone()),
            }
        }
        found
    }
}

impl PartialEq for CArray {
    fn eq(&self, other: &Self) -> bool {
        let mut pairs: Vec<(&CArray, &CArray)> = vec![(self, other)];
        while let Some((left, right)) = pairs.pop() {
            if Rc::ptr_eq(&left.0, &right.0) {
                continue;
            }
            if left.0.len() != right.0.len() {
                return false;
            }
            for (a, b) in left.0.iter().zip(right.0.iter()) {
                match (a, b) {
                    (CVal::Array(a), CVal::Array(b)) => pairs.push((a, b)),
                    (a, b) => {
                        if a != b {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

impl fmt::Debug for CArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::from("[");
        let mut stack: Vec<(&CArray, usize)> = vec![(self, 0)];
        while let Some((array, index)) = stack.pop() {
            let Some(item) = array.0.get(index) else {
                out.push(']');
                continue;
            };
            stack.push((array, index + 1));
            if index > 0 {
                out.push_str(", ");
            }
            match item {
                CVal::Array(inner) => {
                    out.push('[');
                    stack.push((inner, 0));
                }
                leaf => out.push_str(&format!("{leaf:?}")),
            }
        }
        f.write_str(&out)
    }
}

impl Drop for CArray {
    fn drop(&mut self) {
        let mut stack = Vec::new();
        if let Some(items) = Rc::get_mut(&mut self.0) {
            stack.append(items);
        }
        while let Some(item) = stack.pop() {
            if let CVal::Array(mut inner) = item
                && let Some(items) = Rc::get_mut(&mut inner.0)
            {
                stack.append(items);
            }
        }
    }
}
