//! `err` and `ret` make `#[instrument]` rebuild the body differently. The sync
//! form wraps the user's block in a `move ||` closure, which the lint must skip
//! rather than report; its span prologue stays in the outer body. The async
//! form nests an `async move`, so its prologue sits one body deeper and is found
//! only because the detection enters nested bodies. Both must count as
//! instrumented.

#[tracing::instrument(skip_all, err, ret)]
pub fn parse(input: &str) -> Result<u32, String> {
    input.parse().map_err(|_| String::from("not a number"))
}

#[tracing::instrument(err)]
pub async fn parse_later(input: String) -> Result<u32, String> {
    input.parse().map_err(|_| String::from("not a number"))
}
