pub fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

#[cfg(test)]
mod tests {
    use super::add;

    #[hegel::test]
    fn addition_commutes(tc: hegel::TestCase) {
        let a = tc.draw(hegel::generators::integers::<i64>());
        let b = tc.draw(hegel::generators::integers::<i64>());
        assert_eq!(add(a, b), add(b, a));
    }
}
