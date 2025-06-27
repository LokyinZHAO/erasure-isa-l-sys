#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings/isal.rs"));

#[cfg(test)]
mod test {
    use crate::gf_inv;

    #[test]
    fn link_works() {
        // This test is to ensure that the library can be linked correctly.
        // It does not need to do anything, just linking is enough.
        assert_eq!(unsafe { gf_inv(0) }, 0);
    }
}
