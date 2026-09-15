// The crate-level exemption form, against the real driver. Every other
// package here exempts at the item level; this is the only one that proves
// `#![cfg_attr(dylint_lib = "hyp_hegel", allow(...))]` reaches the lint when
// written as an inner attribute on the crate root.
//
// Two separate exemptions are needed, and that is the documented policy
// rather than an oversight: `allow(non_hegel_test)` silences the per-test
// rule but deliberately does *not* suppress `crate_without_hegel_tests`, so a
// crate that has decided against property testing altogether has to say so
// explicitly. Delete either attribute and this package starts failing.
#![cfg_attr(
    dylint_lib = "hyp_hegel",
    allow(
        non_hegel_test,
        reason = "fixed regulatory identifiers checked against a published table; the inputs are an enumerated set, not a domain to sample"
    )
)]
#![cfg_attr(
    dylint_lib = "hyp_hegel",
    allow(
        crate_without_hegel_tests,
        reason = "fixed regulatory identifiers checked against a published table; the inputs are an enumerated set, not a domain to sample"
    )
)]

/// Expand a two-letter country code to its ISO 3166-1 alpha-3 form.
pub fn alpha3(alpha2: &str) -> Option<&'static str> {
    match alpha2 {
        "GB" => Some("GBR"),
        "IE" => Some("IRL"),
        "NZ" => Some("NZL"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::alpha3;

    #[test]
    fn known_codes_expand() {
        assert_eq!(alpha3("GB"), Some("GBR"));
        assert_eq!(alpha3("IE"), Some("IRL"));
        assert_eq!(alpha3("NZ"), Some("NZL"));
    }

    #[test]
    fn unknown_code_is_none() {
        assert_eq!(alpha3("ZZ"), None);
    }
}
