mod analyze;
mod annotation;
mod array_node;
mod attr_item;
mod attr_read;
mod body_instance;
#[cfg(test)]
mod call_result_tests;
mod calls;
#[cfg(test)]
mod cell_tests;
mod cells;
mod check;
mod collect;
mod consistency;
mod core;
mod core_value;
mod coverage;
mod cycles;
mod decl_ref;
mod declined;
mod destinations;
mod detached;
mod env;
mod expand;
mod fact;
mod fold_run;
mod folds;
mod frame;
mod front_end;
mod groups;
mod history;
#[cfg(test)]
mod history_tests;
mod host;
mod inits;
mod instance_row;
mod leaf;
#[cfg(test)]
mod ledger_tests;
mod locals;
mod loop_run;
#[cfg(test)]
mod loop_tests;
mod loops;
mod memo;
mod memo_arg;
mod memo_entry;
mod memo_key;
mod need;
#[cfg(test)]
mod path_tests;
mod paths;
#[cfg(test)]
mod pattern_tests;
mod pick;
mod reason;
mod recording;
mod ref_bodies;
mod references;
mod run;
mod slice;
mod stages;
mod step;
mod store;
#[cfg(test)]
mod to_tests;
mod typing;
mod uses;
mod values;
mod width;
mod wire_row;
#[cfg(test)]
mod wire_tests;
mod wires;
mod work;

pub(crate) use analyze::{Inputs, analyze};
#[cfg(test)]
mod differential_tests;
