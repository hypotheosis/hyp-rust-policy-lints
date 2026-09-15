pub fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn addition_works() {
        assert_eq!(add(2, 2), 4);
    }
}
