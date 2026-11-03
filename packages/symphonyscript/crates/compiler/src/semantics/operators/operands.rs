pub(crate) enum Operands<V> {
    One(V),
    Two(V, V),
    Many(Vec<V>),
}
