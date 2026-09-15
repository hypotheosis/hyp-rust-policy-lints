pub fn render(n: u8) -> String {
    format!("{n:03}")
}

#[cfg(test)]
mod tests {
    use super::render;

    #[cfg_attr(dylint_lib = "hyp_hegel", allow(non_hegel_test))]
    #[test]
    fn golden_format() {
        assert_eq!(render(7), "007");
    }

    #[hegel::test]
    fn always_three_chars(tc: hegel::TestCase) {
        let n = tc.draw(hegel::generators::integers::<u8>());
        assert_eq!(render(n).len(), 3);
    }
}
