pub struct Counter(u32);

impl Counter {
    pub fn get(&self) -> u32 {
        self.0
    }
}
