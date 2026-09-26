pub trait Greet {
    #[tracing::instrument(skip(self))]
    fn greet(&self) -> u32 {
        1
    }
}
