//! A function rewritten by `#[tokio::main]` is checked like any other: the
//! uninstrumented `serve` is reported, at its signature.

#[tokio::main]
pub async fn serve() {}
