#[allow(non_hegel_test, reason = "whole module is example-based by design")]
mod legacy {
    #[test]
    fn one() {
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn two() {
        assert_eq!(2 + 2, 4);
    }
}

fn main() {}
