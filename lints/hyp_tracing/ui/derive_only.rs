//! Derived methods come from an external macro expansion: no source line to
//! annotate, and rustc cancels the diagnostic because `uninstrumented_fn` does
//! not opt into `report_in_external_macro`.

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: i32,
}
