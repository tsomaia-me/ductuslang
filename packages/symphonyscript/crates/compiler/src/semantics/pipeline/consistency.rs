use std::collections::BTreeMap;

use super::core::Core;
use super::core_value::CoreValue;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::operators::Value;

fn integer(class: usize) -> bool {
    matches!(class, 0 | 4..=6)
}

type Reads = Vec<(u32, usize, Vec<u32>, CoreValue)>;

impl Core<'_> {
    pub fn consistency(&mut self) -> Result<(), Internal> {
        let mut by_target: BTreeMap<(bool, u32), Reads> = BTreeMap::new();
        for ((read, class, chain), (target, value)) in &self.uses {
            CountedWalk::CoreUse.step();
            if *class != 3 {
                by_target.entry(*target).or_default().push((
                    *read,
                    *class,
                    chain.clone(),
                    value.clone(),
                ));
            }
        }
        for (_, uses) in by_target {
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
                for call in float_chain.iter().rev() {
                    related.push((self.node_span(*call)?, "in this call".to_string()));
                }
            }
            for call in int_chain.iter().rev() {
                related.push((self.node_span(*call)?, "in this call".to_string()));
            }
            self.draw_related(
                Code::InconsistentUses,
                span,
                related,
                "this value is an integer here and a different number at its float use",
            );
        }
        Ok(())
    }

    fn same_number(&mut self, int: &CoreValue, float: &CoreValue) -> bool {
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
