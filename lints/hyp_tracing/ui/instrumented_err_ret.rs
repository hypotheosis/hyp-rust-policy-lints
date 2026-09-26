//! `err` and `ret` make `#[instrument]` rebuild the body differently -- the
//! sync form wraps the user's block in a `move ||` closure, the async form in a
//! nested `async move` -- so the prologue the detection looks for sits one body
//! deeper. Both must still count as instrumented.

#[tracing::instrument(skip_all, err, ret)]
pub fn parse(input: &str) -> Result<u32, String> {
    input.parse().map_err(|_| String::from("not a number"))
}

#[tracing::instrument(err)]
pub async fn parse_later(input: String) -> Result<u32, String> {
    input.parse().map_err(|_| String::from("not a number"))
}
