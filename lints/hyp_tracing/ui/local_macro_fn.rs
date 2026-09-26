macro_rules! make_fn {
    ($name:ident) => {
        pub fn $name() {}
    };
}

make_fn!(generated);
