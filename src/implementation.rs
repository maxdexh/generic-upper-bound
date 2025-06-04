include!(concat!(env!("OUT_DIR"), "/for_each_size.rs"));
use for_each_size;

#[track_caller]
#[cold]
const fn unreachable() -> ! {
    unreachable!()
}

use crate::{const_value, AcceptUpperBound, Impl};

impl<F: AcceptUpperBound> Impl<F> {
    // encourage the compiler to cache the result by promoting
    const DESIRED_REF: &'static usize = &F::DESIRED_GENERIC;
    pub const DESIRED: usize = *Self::DESIRED_REF;

    pub const ACTUAL: usize = 'ret: {
        let desired = F::DESIRED_GENERIC;
        macro_rules! check_size {
            ($($n:tt)*) => {$(
                if $n >= desired {
                    break 'ret $n;
                }
            )*};
        }
        for_each_size! { check_size }
        unreachable()
    };

    pub const EVAL: F::Output = 'ret: {
        let actual = Self::ACTUAL;
        macro_rules! check_size {
            ($($n:tt)*) => {$(
                if $n == actual {
                    // SAFETY: This is only evaluated for the actual value of the const,
                    // which returns init
                    break 'ret const_value::<F::Eval<$n>>();
                }
            )*};
        }
        for_each_size! { check_size }
        unreachable()
    };
}
