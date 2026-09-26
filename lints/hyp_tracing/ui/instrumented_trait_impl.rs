pub trait Greet {
    fn greet(&self) -> u32;
}

pub struct English;

impl Greet for English {
    #[tracing::instrument(skip(self))]
    fn greet(&self) -> u32 {
        1
    }
}
