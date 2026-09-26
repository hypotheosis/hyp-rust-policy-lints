//! Every function instrumented: sync and async, free and associated, a trait
//! default method, and functions rewritten by `#[tokio::main]` and
//! `#[async_trait]`. Nothing here should ever produce a diagnostic.

#[tracing::instrument]
pub fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

#[tracing::instrument]
pub async fn add_later(a: i64, b: i64) -> i64 {
    add(a, b)
}

pub trait Describe {
    #[tracing::instrument(skip(self))]
    fn describe(&self) -> String {
        String::from("widget")
    }
}

pub struct Widget;

impl Widget {
    #[tracing::instrument(skip(self))]
    pub fn id(&self) -> u32 {
        7
    }
}

impl Describe for Widget {}

// `#[tokio::main]` with `#[tracing::instrument]` in either order: both are
// detected. The README recommends the first.
#[tracing::instrument]
pub fn run() {
    serve();
    serve_outer();
}

#[tokio::main]
#[tracing::instrument]
async fn serve() {}

#[tracing::instrument]
#[tokio::main]
async fn serve_outer() {}

// `#[tracing::instrument]` on the methods inside an `#[async_trait]` trait
// and impl: detected.
#[async_trait::async_trait]
pub trait Fetch {
    async fn fetch(&self) -> u32;

    #[tracing::instrument(skip(self))]
    async fn fetch_twice(&self) -> u32 {
        self.fetch().await * 2
    }
}

#[async_trait::async_trait]
impl Fetch for Widget {
    #[tracing::instrument(skip(self))]
    async fn fetch(&self) -> u32 {
        7
    }
}
