//! A package that satisfies the policy: it has tests, and every one of them is
//! a hegel property test. Nothing here should ever produce a diagnostic.

pub fn min(a: i64, b: i64) -> i64 {
    if a < b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::min;

    #[hegel::test]
    fn min_is_commutative(tc: hegel::TestCase) {
        let a = tc.draw(hegel::generators::integers::<i64>());
        let b = tc.draw(hegel::generators::integers::<i64>());
        assert_eq!(min(a, b), min(b, a));
    }

    #[hegel::test]
    fn min_returns_an_argument(tc: hegel::TestCase) {
        let a = tc.draw(hegel::generators::integers::<i64>());
        let b = tc.draw(hegel::generators::integers::<i64>());
        let m = min(a, b);
        assert!(m == a || m == b);
    }
}
