use super::column::{Column, TripleSet, dense};
use super::core::Core;
use super::core_value::CoreValue;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::NONE;
use crate::semantics::operators::Value;

fn integer(class: usize) -> bool {
    matches!(class, 0 | 4..=6)
}

type Reads = Vec<(u32, usize, (Vec<u32>, Vec<u32>), CoreValue)>;

pub(crate) struct UseRow {
    target: (bool, u32),
    value: CoreValue,
    chain: Vec<u32>,
    links: Vec<u32>,
}

impl Core<'_> {
    fn use_path(&mut self) -> u32 {
        let chain = self.eval_chain.id();
        self.use_paths
            .path(std::iter::once(chain).chain(self.use_links.iter().map(|(item, _)| *item)))
    }

    pub fn record_use(&mut self, read: u32, class: usize, target: (bool, u32), value: CoreValue) {
        let key = (read, dense(class), self.use_path());
        if self.uses.get(&key).is_some() {
            return;
        }
        let row = UseRow {
            target,
            value,
            chain: self.eval_chain.to_vec(),
            links: self.use_links.iter().map(|(item, _)| *item).collect(),
        };
        self.uses.insert(key, row);
    }

    pub fn consistency(&mut self) -> Result<(), Internal> {
        let mut by_target: [Column<Reads>; 2] = Default::default();
        for ((read, class, _), row) in self.uses.iter() {
            CountedWalk::CoreUse.step();
            let class = class as usize;
            if class != 3
                && let Some(column) = by_target.get_mut(usize::from(row.target.0))
            {
                column.entry(row.target.1).or_default().push((
                    read,
                    class,
                    (row.chain.clone(), row.links.clone()),
                    row.value.clone(),
                ));
            }
        }
        for mut uses in by_target.into_iter().flat_map(Column::into_values) {
            uses.sort_by(|left, right| {
                (left.0, left.1, &left.2).cmp(&(right.0, right.1, &right.2))
            });
            let Some((int_read, _, int_chain, int_value)) = uses
                .iter()
                .find(|(_, class, _, _)| integer(*class))
                .cloned()
            else {
                continue;
            };
            let mut float = None;
            for (read, class, chain, value) in &uses {
                if !integer(*class)
                    && (*read != int_read || *chain != int_chain)
                    && !self.same_number(&int_value, value)
                {
                    float = Some((*read, chain.clone()));
                    break;
                }
            }
            let Some((float_read, float_chain)) = float else {
                continue;
            };
            let span = self.node_span(int_read)?;
            let float_span = self.node_span(float_read)?;
            let mut related = Vec::new();
            if float_span != span {
                related.push((float_span, "the float use".to_string()));
            }
            if float_chain != int_chain {
                self.chain_links(&float_chain, &mut related)?;
            }
            self.chain_links(&int_chain, &mut related)?;
            self.draw_related(
                Code::InconsistentUses,
                span,
                related,
                "this value is an integer here and a different number at its float use",
            );
        }
        self.loop_consistency()
    }

    fn loop_consistency(&mut self) -> Result<(), Internal> {
        let mut by_env: Column<Vec<(u32, usize, CoreValue)>> = Column::new();
        for ((env, read, class), value) in self.loop_reads.iter() {
            CountedWalk::CoreUse.step();
            by_env
                .entry(env)
                .or_default()
                .push((read, class as usize, value.clone()));
        }
        let mut drawn = TripleSet::default();
        for (env, reads) in by_env.iter() {
            let Some((int_read, _, int_value)) =
                reads.iter().find(|(_, class, _)| integer(*class)).cloned()
            else {
                continue;
            };
            let mut float = None;
            for (read, class, value) in reads {
                if !integer(*class) && *read != int_read && !self.same_number(&int_value, value) {
                    float = Some(*read);
                    break;
                }
            }
            let Some(float_read) = float else {
                continue;
            };
            let saved = self.enter_env(env);
            let links = self
                .use_paths
                .path(std::iter::once(NONE).chain(self.use_links.iter().map(|(item, _)| *item)));
            if drawn.insert((int_read, float_read, links)) {
                let span = self.node_span(int_read)?;
                let float_span = self.node_span(float_read)?;
                self.draw_related(
                    Code::InconsistentUses,
                    span,
                    vec![(float_span, "the float use".to_string())],
                    "this value is an integer here and a different number at its float use",
                );
            }
            self.leave_env(saved);
        }
        Ok(())
    }

    pub fn same_number(&mut self, int: &CoreValue, float: &CoreValue) -> bool {
        let mut pairs = vec![(int.clone(), float.clone())];
        while let Some(pair) = pairs.pop() {
            CountedWalk::CoreUse.step();
            match pair {
                (Value::Array(ints), Value::Array(floats)) => {
                    let length = self.store.len(ints);
                    if length != self.store.len(floats) {
                        return false;
                    }
                    for position in 0..length as usize {
                        match (
                            self.store.element(ints, position),
                            self.store.element(floats, position),
                        ) {
                            (Some(int), Some(float)) => pairs.push((int, float)),
                            _ => return false,
                        }
                    }
                }
                (Value::Int(value), Value::F32(other)) => {
                    if (value as f32).to_bits() != other.to_bits() {
                        return false;
                    }
                }
                (Value::Int(value), Value::F64(other)) => {
                    if (value as f64).to_bits() != other.to_bits() {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }
}

impl Core<'_> {
    fn chain_links(
        &self,
        (calls, uses): &(Vec<u32>, Vec<u32>),
        related: &mut Vec<(symphonyscript_core::Span, String)>,
    ) -> Result<(), Internal> {
        for call in calls.iter().rev() {
            related.push((self.node_span(*call)?, "in this call".to_string()));
        }
        for item in uses {
            if let Some(span) = self.use_spans.get(item) {
                related.push((span.clone(), "in this use of the pattern".to_string()));
            }
        }
        Ok(())
    }
}
