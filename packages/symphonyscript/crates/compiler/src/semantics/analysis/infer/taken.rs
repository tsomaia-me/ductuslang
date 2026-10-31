pub(super) enum Taken<T> {
    Known(T),
    Faulty,
    Runtime,
}
