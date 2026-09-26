//! Every function instrumented: sync and async, free and associated, and a
//! trait default method. Nothing here should ever produce a diagnostic.

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
