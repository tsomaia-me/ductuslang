mod analyze;
mod annotation;
mod array_node;
mod attr_item;
mod attr_read;
mod body_instance;
mod bound_form;
mod bounds;
mod bundle_fors;
mod call_chain;
#[cfg(test)]
mod call_result_tests;
mod calls;
mod capacities;
#[cfg(test)]
mod capacity_tests;
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
mod dot_calls;
#[cfg(test)]
mod enum_tests;
mod enums;
mod env;
mod expand;
mod extent;
mod extreme;
mod fact;
mod families;
#[cfg(test)]
mod flip_tests;
mod float_fit;
mod fold_run;
mod folds;
mod frame;
mod front_end;
#[cfg(test)]
mod gate_tests;
mod gates;
mod groups;
mod history;
#[cfg(test)]
mod history_tests;
mod host;
#[cfg(test)]
mod init_fit_tests;
mod init_read;
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
mod piecewise;
mod ratio;
mod reason;
mod recording;
mod ref_bodies;
mod references;
mod rule_width;
mod run;
mod slice;
mod slice_transform;
mod stages;
mod step;
mod store;
mod suffix;
#[cfg(test)]
mod suffix_tests;
#[cfg(test)]
mod to_tests;
mod typing;
mod unit_conflict;
mod unit_reach;
#[cfg(test)]
mod unit_tests;
mod unread_arg;
mod uses;
mod value_keys;
mod values;
mod whens;
mod width;
mod wire_names;
mod wire_row;
#[cfg(test)]
mod wire_tests;
mod wires;
mod work;

pub(crate) use analyze::{Inputs, analyze};
#[cfg(test)]
mod differential_tests;
