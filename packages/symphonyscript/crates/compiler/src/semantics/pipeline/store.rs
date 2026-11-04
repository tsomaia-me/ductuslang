use super::array_node::ArrayNode;
use super::core_value::CoreValue;
use crate::diagnostics::Code;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::operators::Value;

const LENGTH_LIMIT: u64 = 1 << 31;

#[derive(Default)]
pub(crate) struct Store {
    nodes: Vec<ArrayNode>,
    layouts: Vec<Vec<CoreValue>>,
}

impl Store {
    pub fn literal(&mut self, items: Vec<CoreValue>) -> u32 {
        let len = u32::try_from(items.len()).unwrap_or(u32::MAX);
        let layout = u32::try_from(self.layouts.len()).unwrap_or(u32::MAX);
        self.layouts.push(items);
        CountedWalk::DenseLayout.step();
        self.push(ArrayNode::Dense {
            layout,
            start: 0,
            len,
        })
    }

    fn push(&mut self, node: ArrayNode) -> u32 {
        CountedWalk::JoinNode.step();
        let id = u32::try_from(self.nodes.len()).unwrap_or(u32::MAX);
        self.nodes.push(node);
        id
    }

    pub fn len(&self, array: u32) -> u32 {
        match self.nodes.get(array as usize) {
            Some(
                ArrayNode::Dense { len, .. }
                | ArrayNode::Join { len, .. }
                | ArrayNode::Slice { len, .. },
            ) => *len,
            None => 0,
        }
    }

    pub fn row(&mut self, array: u32) -> Option<Vec<u32>> {
        if self.len(array) == 0 {
            return None;
        }
        let first = self.element(array, 0)?;
        Some(self.lengths(&first))
    }

    pub fn lengths(&mut self, value: &CoreValue) -> Vec<u32> {
        let mut lengths = Vec::new();
        let mut current = value.clone();
        while let Value::Array(array) = current {
            let len = self.len(array);
            lengths.push(len);
            if len == 0 {
                break;
            }
            match self.element(array, 0) {
                Some(first) => current = first,
                None => break,
            }
        }
        lengths
    }

    pub fn elements(&mut self, value: &CoreValue) -> u64 {
        self.lengths(value)
            .iter()
            .fold(1u64, |total, len| total.saturating_mul(u64::from(*len)))
    }

    pub fn join(&mut self, left: u32, right: u32) -> Result<u32, Code> {
        match (self.row(left), self.row(right)) {
            (Some(a), Some(b)) if a != b => return Err(Code::RaggedArray),
            _ => {}
        }
        let len = u64::from(self.len(left)) + u64::from(self.len(right));
        if len >= LENGTH_LIMIT {
            return Err(Code::LengthLimit);
        }
        let len = u32::try_from(len).map_err(|_| Code::LengthLimit)?;
        if let Some(ArrayNode::Dense {
            layout,
            start,
            len: left_len,
        }) = self.nodes.get(left as usize).copied()
        {
            let newest = self
                .layouts
                .get(layout as usize)
                .is_some_and(|items| items.len() == (start + left_len) as usize);
            if newest {
                let mut added = Vec::new();
                self.walk(right, &mut added);
                if let Some(items) = self.layouts.get_mut(layout as usize) {
                    items.extend(added);
                }
                return Ok(self.push(ArrayNode::Dense { layout, start, len }));
            }
        }
        Ok(self.push(ArrayNode::Join { left, right, len }))
    }

    pub fn slice(&mut self, base: u32, start: usize, end: usize) -> u32 {
        let start = u32::try_from(start).unwrap_or(u32::MAX);
        let end = u32::try_from(end).unwrap_or(u32::MAX);
        let len = end.saturating_sub(start);
        if let Some(ArrayNode::Dense {
            layout,
            start: from,
            ..
        }) = self.nodes.get(base as usize).copied()
        {
            return self.push(ArrayNode::Dense {
                layout,
                start: from + start,
                len,
            });
        }
        self.push(ArrayNode::Slice { base, start, len })
    }

    pub fn element(&mut self, array: u32, position: usize) -> Option<CoreValue> {
        let position = u32::try_from(position).ok()?;
        if position >= self.len(array) {
            return None;
        }
        let (layout, start) = self.dense(array);
        self.layouts
            .get(layout as usize)?
            .get((start + position) as usize)
            .cloned()
    }

    fn dense(&mut self, array: u32) -> (u32, u32) {
        if let Some(ArrayNode::Dense { layout, start, .. }) = self.nodes.get(array as usize) {
            return (*layout, *start);
        }
        let mut items = Vec::new();
        self.walk(array, &mut items);
        let len = u32::try_from(items.len()).unwrap_or(u32::MAX);
        let layout = u32::try_from(self.layouts.len()).unwrap_or(u32::MAX);
        self.layouts.push(items);
        CountedWalk::DenseLayout.step();
        if let Some(node) = self.nodes.get_mut(array as usize) {
            *node = ArrayNode::Dense {
                layout,
                start: 0,
                len,
            };
        }
        (layout, 0)
    }

    fn walk(&self, array: u32, out: &mut Vec<CoreValue>) {
        let mut stack = vec![(array, 0u32, self.len(array))];
        while let Some((node, skip, take)) = stack.pop() {
            CountedWalk::CursorStep.step();
            if take == 0 {
                continue;
            }
            match self.nodes.get(node as usize).copied() {
                Some(ArrayNode::Dense { layout, start, .. }) => {
                    let from = (start + skip) as usize;
                    if let Some(items) = self
                        .layouts
                        .get(layout as usize)
                        .and_then(|items| items.get(from..from + take as usize))
                    {
                        out.extend(items.iter().cloned());
                    }
                }
                Some(ArrayNode::Join { left, right, .. }) => {
                    let left_len = self.len(left);
                    if skip >= left_len {
                        stack.push((right, skip - left_len, take));
                    } else {
                        let from_left = (left_len - skip).min(take);
                        stack.push((right, 0, take - from_left));
                        stack.push((left, skip, from_left));
                    }
                }
                Some(ArrayNode::Slice { base, start, .. }) => {
                    stack.push((base, start + skip, take));
                }
                None => {}
            }
        }
    }
}
