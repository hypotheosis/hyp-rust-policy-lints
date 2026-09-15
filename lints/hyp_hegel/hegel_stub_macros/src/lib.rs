//! Minimal stand-in for the real hegel attribute macros, used only by UI
//! fixtures.
//!
//! Expands `#[hegel::test]` into a plain `#[test]` with the `tc` parameter
//! dropped, which is enough to reproduce the macro-expansion chain the lint
//! inspects.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = item.to_string();
    let name = input
        .split("fn ")
        .nth(1)
        .and_then(|rest| rest.split('(').next())
        .unwrap_or("generated")
        .trim()
        .to_string();
    let open = input.find('{').expect("test function must have a body");
    let body = &input[open + 1..input.rfind('}').expect("unbalanced body")];
    format!("#[test] fn {name}() {{ let tc = hegel::TestCase; let _ = &tc; {body} }}")
        .parse()
        .unwrap()
}
