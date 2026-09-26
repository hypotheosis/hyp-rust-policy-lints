pub struct Counter(u32);

impl Counter {
    #[tracing::instrument(skip(self))]
    pub fn get(&self) -> u32 {
        self.0
    }
}
