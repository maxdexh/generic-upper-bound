#![no_std]
// reason: flagged in macro generated code
#![allow(clippy::absurd_extreme_comparisons)]
#![allow(unused_comparisons)]
#![allow(rustdoc::redundant_explicit_links)]

//! This crate allows performing const calculations with the help of a generic const `usize`
//! that is a reasonable upper bound of some desired associated const `usize`.
//!
//! The API of this crate is structed as follows:
//! - [`AcceptUpperBound`](crate::AcceptUpperBound) is the heart of this crate. Implementors use it to specify which
//!   generic const they want to be passed to them and what to do with any given upper bound for it.
//!   It can be implemented conveniently using [`impl_accept_upper_bound!`](crate::impl_accept_upper_bound).
//! - [`eval_with_upper_bound`](crate::eval_with_upper_bound) is used to get the result of evaluating an upper bound acceptor
//!   with the best-effort upper bound that this crate can offer.
//!
//! While you cannot use this to write a function with a signature that returns e.g. `[T; M + N]`
//! with generic `M` and `N`, you can use it to temporarily get an array of size `M + N`, use it
//! to do something useful, then return the result of that computation.
//! For example, you can concatenate two strings at compile time, even if their value is dependent
//! on generic parameters:
//! ```
//! use generic_upper_bound as gub;
//! pub trait MyTrait {
//!     const SOME_STR: &'static str;
//! }
//! struct Concat<A, B>(A, B);
//! gub::impl_accept_upper_bound! {
//!     impl{A: MyTrait, B: MyTrait} Concat<A, B>;
//!
//!     const DESIRED_GENERIC: usize = A::SOME_STR.len() + B::SOME_STR.len();
//!
//!     const EVAL<const UPPER: usize>: &'static [u8] = &{
//!         let l = A::SOME_STR.as_bytes();
//!         let r = B::SOME_STR.as_bytes();
//!         let mut out = [0; UPPER];
//!         let mut off = 0;
//!         // after 1.86, use split_at_mut and copy_from_slice
//!         let mut i = 0;
//!         while i < l.len() {
//!             out[off] = l[i];
//!             off += 1;
//!             i += 1;
//!         }
//!         i = 0;
//!         while i < r.len() {
//!             out[off] = r[i];
//!             off += 1;
//!             i += 1;
//!         }
//!         out
//!     };
//! }
//! impl<A: MyTrait, B: MyTrait> MyTrait for (A, B) {
//!     // evaluate the upper bound acceptor, trim trailing nul bytes
//!     // and convert to string
//!     const SOME_STR: &'static str = match core::str::from_utf8(
//!         gub::eval_with_upper_bound::<Concat<A, B>>()
//!             .split_at(gub::desired_generic::<Concat<A, B>>())
//!             .0,
//!     ) {
//!         Ok(s) => s,
//!         _ => unreachable!(),
//!     };
//! }
//! impl MyTrait for () {
//!     const SOME_STR: &'static str = "ABC";
//! }
//! impl MyTrait for i32 { const SOME_STR: &'static str = "123";
//! }
//! let concatenated: &'static str = <((), i32)>::SOME_STR;
//! assert_eq!(concatenated, "ABC123");
//! ```
//! Note that this example can be generalized and optimized. For instance, it is possible to accept
//! any `&'a [&'b str]` as input and this will also be more efficient (most of the time)
//! due to the overhead from the inexact upper bound used for each concatenation (which will
//! likely affect the final binary size).
//!
//! See the [`const-util`](https://docs.rs/const-util/latest/const_util/) crate for an
//! implementation of this.
//!
//! # MSRV
//! The MSRV is 1.78. This is to allow this crate to be used as a workaround for the breaking change
//! to const promotion that was introduced by that version.

pub extern crate type_const;
pub use type_const::{value_of as const_value, Const, TypeOf};

/// Allows implementing a callback pattern that accepts an upper bound for a desired generic const
/// parameter.
///
/// The trait works by allowing implementors to specify, for each upper bound, a value that is
/// computed from that upper bound. This value is represented as a generic associated type with
/// bound [`Const`], as this is currently the only way to implement a `const` item with generics
/// in a trait.
///
/// When passed to [`eval_with_upper_bound`], [`Eval::<UPPER>::VALUE`](Const::VALUE) will be evaluated
/// with a parameter `UPPER` that satisfies `DESIRED_GENERIC <= UPPER < 2 * DESIRED_GENERIC`.
pub trait AcceptUpperBound {
    /// The output type of the evaluation.
    type Output;

    /// The desired value and lower bound that the implementor wants to be passed to [`Self::Eval`].
    const DESIRED_GENERIC: usize;

    /// Evals the constant by mapping a generic parameter that is at least the desired value
    /// to the output value. `const_value::<Eval<UPPER>>()` should be indistinguishable for
    /// all *possible* generic parameters passed to this.
    type Eval<const UPPER: usize>: Const<Type = Self::Output>;
}

struct Impl<A>(A);

mod implementation;

/// Returns [`AcceptUpperBound::DESIRED_GENERIC`].
pub const fn desired_generic<A: AcceptUpperBound>() -> usize {
    Impl::<A>::DESIRED
}

/// Returns the parameter that [`eval_with_upper_bound`] passes to [`AcceptUpperBound::Eval`].
pub const fn get_upper_bound<A: AcceptUpperBound>() -> usize {
    Impl::<A>::ACTUAL
}

/// Evaluates [`AcceptUpperBound`].
///
/// In the language of `generic_const_exprs`, this function returns
/// `const_value::<F::Eval<{ get_upper_bound::<F>() }>>()`
pub const fn eval_with_upper_bound<A: AcceptUpperBound>() -> A::Output {
    Impl::<A>::EVAL
}

/// Implements [`AcceptUpperBound`] by generating a hidden [`Const`] implementor.
///
/// Generic parameters are passed in braces (`{...}`) after `impl` and cannot have a trailing
/// comma. Where bounds are optionally passed in braces after the implementing type.
///
/// The example from the [crate level documentation](crate) can be written manually like this:
/// ```
/// use generic_upper_bound as gub;
/// pub trait MyTrait {
///     const SOME_STR: &'static str;
/// }
/// impl<A: MyTrait, B: MyTrait> MyTrait for (A, B) {
///     const SOME_STR: &'static str = match core::str::from_utf8(
///         gub::eval_with_upper_bound::<Concat<A, B>>()
///             .split_at(gub::desired_generic::<Concat<A, B>>())
///             .0,
///     ) {
///         Ok(s) => s,
///         _ => unreachable!(),
///     };
/// }
///
/// struct Concat<A, B>(A, B);
/// impl<A: MyTrait, B: MyTrait> gub::AcceptUpperBound for Concat<A, B> {
///     type Output = &'static [u8];
///     // Want to be passed at least the total length of the strings
///     const DESIRED_GENERIC: usize = A::SOME_STR.len() + B::SOME_STR.len();
///     // Decide on what to do with each generic const
///     type Eval<const UPPER: usize> = ConcatImpl<A, B, UPPER>;
/// }
/// struct ConcatImpl<A, B, const N: usize>(A, B);
/// impl<A: MyTrait, B: MyTrait, const N: usize> gub::Const for ConcatImpl<A, B, N> {
///     type Type = &'static [u8];
///     const VALUE: Self::Type = panic!("...");
/// }
/// ```
#[macro_export]
macro_rules! impl_accept_upper_bound {
    {
        $(#[$meta:meta])*
        impl{$($params:tt)*} $Self:ty $({ $($where_bounds:tt)* })?;

        const DESIRED_GENERIC: $usize_d:ty = $DESIRED_GENERIC:expr;
        const EVAL<const $UPPER:ident: $usize_e:ty>: $Output:ty = $EVAL:expr;

    } => {
        const _: () = {
            pub struct __Eval<__Eval, const $UPPER: $usize_e>(__Eval);
            impl<$($params)*, const $UPPER: $usize_e> $crate::Const for __Eval<$Self, $UPPER> $($($where_bounds)*)? {
                type Type = $Output;
                const VALUE: Self::Type = $EVAL;
            }
            $(#[$meta])*
            impl<$($params)*> $crate::AcceptUpperBound for $Self $($($where_bounds)*)? {
                type Output = $Output;
                const DESIRED_GENERIC: $usize_d = $DESIRED_GENERIC;
                type Eval<const $UPPER: $usize_e> = __Eval<Self, $UPPER>;
            }
        };
    };
}
