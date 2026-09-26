pub struct Point {
    x: i32,
}

#[allow(uninstrumented_fn, reason = "trivial accessors on a hot path")]
impl Point {
    pub fn x(&self) -> i32 {
        self.x
    }
}
