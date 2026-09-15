mod legacy {
    #![allow(non_hegel_test, reason = "whole module is example-based by design")]

    #[test]
    fn inherits_the_module_exemption() {
        assert_eq!(1 + 1, 2);
    }

    #[deny(non_hegel_test)]
    #[test]
    fn opts_back_in() {
        assert_eq!(2 + 2, 4);
    }
}

fn main() {}
