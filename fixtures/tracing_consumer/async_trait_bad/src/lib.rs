//! Methods rewritten by `#[async_trait]` are checked like any other: the
//! uninstrumented `fetch` is reported, at the `#[async_trait]` attribute.

#[async_trait::async_trait]
pub trait Fetch {
    async fn fetch(&self) -> u32;
}

pub struct Widget;

#[async_trait::async_trait]
impl Fetch for Widget {
    async fn fetch(&self) -> u32 {
        7
    }
}
