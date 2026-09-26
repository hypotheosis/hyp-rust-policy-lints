//! A trait method with no body has nothing to instrument.

pub trait Shape {
    fn area(&self) -> f64;
}
