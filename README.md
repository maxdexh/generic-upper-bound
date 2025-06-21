[![Crates.io](https://img.shields.io/crates/v/generic-upper-bound.svg)](https://crates.io/crates/generic-upper-bound)
[![Documentation](https://docs.rs/generic-upper-bound/badge.svg)](https://docs.rs/generic-upper-bound)
[![Rust](https://img.shields.io/badge/rust-1.78.0%2B-blue.svg?maxAge=3600)](https://github.com/rust-lang/generic-upper-bound)

<!-- cargo-rdme start -->

This crate allows performing const calculations with the help of a generic const `usize`
that is a reasonable upper bound of some desired associated const `usize`.

The API of this crate is structed as follows:
- [`AcceptUpperBound`](https://docs.rs/generic-upper-bound/latest/generic_upper_bound/trait.AcceptUpperBound.html) is the heart of this crate. Implementors use it to specify which
  generic const they want to be passed to them and what to do with any given upper bound for it.
  It can be implemented conveniently using [`impl_accept_upper_bound!`](https://docs.rs/generic-upper-bound/latest/generic_upper_bound/macro.impl_accept_upper_bound.html).
- [`eval_with_upper_bound`](https://docs.rs/generic-upper-bound/latest/generic_upper_bound/fn.eval_with_upper_bound.html) is used to get the result of evaluating an upper bound acceptor
  with the best-effort upper bound that this crate can offer.

While you cannot use this to write a function with a signature that returns e.g. `[T; M + N]`
with generic `M` and `N`, you can use it to temporarily get an array of size `M + N`, use it
to do something useful, then return the result of that computation.
For example, you can concatenate two strings at compile time, even if their value is dependent
on generic parameters:
```rust
use generic_upper_bound as gub;
pub trait MyTrait {
    const SOME_STR: &'static str;
}
struct Concat<A, B>(A, B);
gub::impl_accept_upper_bound! {
    impl{A: MyTrait, B: MyTrait} Concat<A, B>;

    const DESIRED_GENERIC: usize = A::SOME_STR.len() + B::SOME_STR.len();

    const EVAL<const UPPER: usize>: &'static [u8] = &{
        let l = A::SOME_STR.as_bytes();
        let r = B::SOME_STR.as_bytes();
        let mut out = [0; UPPER];
        let mut off = 0;
        // after 1.86, use split_at_mut and copy_from_slice
        let mut i = 0;
        while i < l.len() {
            out[off] = l[i];
            off += 1;
            i += 1;
        }
        i = 0;
        while i < r.len() {
            out[off] = r[i];
            off += 1;
            i += 1;
        }
        out
    };
}
impl<A: MyTrait, B: MyTrait> MyTrait for (A, B) {
    // evaluate the upper bound acceptor, trim trailing nul bytes
    // and convert to string
    const SOME_STR: &'static str = match core::str::from_utf8(
        gub::eval_with_upper_bound::<Concat<A, B>>()
            .split_at(gub::desired_generic::<Concat<A, B>>())
            .0,
    ) {
        Ok(s) => s,
        _ => unreachable!(),
    };
}
impl MyTrait for () {
    const SOME_STR: &'static str = "ABC";
}
impl MyTrait for i32 {
    const SOME_STR: &'static str = "123";
}
let concatenated: &'static str = <((), i32)>::SOME_STR;
assert_eq!(concatenated, "ABC123");
```
Note that this example can be generalized and optimized. For instance, it is possible to accept
any `&'a [&'b str]` as input and this will also be more efficient (most of the time)
due to the overhead from the inexact upper bound used for each concatenation (which will
likely affect the final binary size).

See the [`const-util`](https://docs.rs/const-util/latest/const_util/) crate for an
implementation of this.

# MSRV
The MSRV is 1.78. This is to allow this crate to be used as a workaround for the breaking change
to const promotion that was introduced by that version.

<!-- cargo-rdme end -->
