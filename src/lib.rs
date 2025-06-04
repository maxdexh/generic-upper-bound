#![no_std]
// reason: flagged in macro generated code
#![allow(clippy::absurd_extreme_comparisons)]
#![allow(unused_comparisons)]

//! Provides functionality to get a const generic `usize` that is that is a reasonable
//! upper bound for a specified associated const `usize` for the purpose of intermediate
//! const calculations, as a workaround for `generic_const_exprs`.
//!
//! The API of this crate is structed as follows:
//! - [`AcceptUpperBound`] is the heart of this crate. Implementors use it to specify which
//!   generic const they want to be passed and what to do with any given upper bound for it.
//! - [`eval_with_upper_bound`] is used to get the result of evaluating an upper bound acceptor
//!   with the best-effort upper bound that this crate can offer.
//!
//! While you cannot use this to write a function with a signature that returns e.g. `[T; M + N]`
//! with generic `M` and `N`, you can use it to temporarily get an array of size `M + N`, use it
//! to do something useful, then return the result of that computation.
//! For example, you can concatenate two strings at compile time, even if their value is dependent
//! on generic paramters:
//! ```
//! use generic_upper_bound as gub;
//! pub trait MyTrait {
//!     const SOME_STR: &'static str;
//! }
//! impl<A, B> MyTrait for (A, B)
//! where
//!     A: MyTrait,
//!     B: MyTrait,
//! {
//!     // evaluate our upper bound acceptor to implement concatenation
//!     const SOME_STR: &'static str = {
//!         let slice: &'static [u8] = gub::eval_with_upper_bound::<Concat<A, B>>();
//!
//!         // take subslice without trailing zeros and convert to string
//!         let total_length = gub::desired_generic::<Concat<A, B>>();
//!         match core::str::from_utf8(slice.split_at(total_length).0) {
//!             Ok(s) => s,
//!             _ => unreachable!(),
//!         }
//!     };
//! }
//!
//! struct Concat<A, B>(A, B);
//! impl<A: MyTrait, B: MyTrait> gub::AcceptUpperBound for Concat<A, B> {
//!     type Output = &'static [u8];
//!     // Want to be passed at least the total length of the strings
//!     const DESIRED_GENERIC: usize = A::SOME_STR.len() + B::SOME_STR.len();
//!     // Decide on what to do with each generic const
//!     type Eval<const UPPER: usize> = ConcatImpl<A, B, UPPER>;
//! }
//!
//! struct ConcatImpl<A, B, const N: usize>(A, B);
//! impl<A, B, const N: usize> gub::Const for ConcatImpl<A, B, N>
//! where
//!     A: MyTrait,
//!     B: MyTrait,
//! {
//!     type Type = &'static [u8];
//!     // Write the bytes into `[u8; N]` and promote the result
//!     const VALUE: Self::Type = &{
//!         let l = A::SOME_STR.as_bytes();
//!         let r = B::SOME_STR.as_bytes();
//!         let mut out = [0; N];
//!         let mut off = 0;
//!         let mut i = 0; // in >=1.86, you can use split_at_mut and copy_from_slice
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
//!
//! impl MyTrait for () {
//!     const SOME_STR: &'static str = "ABC";
//! }
//! impl MyTrait for i32 {
//!     const SOME_STR: &'static str = "123";
//! }
//! let concatenated: &'static str = <((), i32)>::SOME_STR;
//! assert_eq!(concatenated, "ABC123");
//! ```
//! Note that this example can be generalized and optimized. For instance, it is possible to accept
//! any `&'a [&'b str]` where `'b: 'a` as input and this will also be more efficient (most of the
//! time) due to the overhead from the inexact upper bound used for each concatenation (which will
//! likely affect the final binary size).

/// A trait for a type that holds a value.
pub trait Const {
    /// The type of the const
    type Type;
    /// The implementation of the value of the const. Use [`const_value`] for accessing the value.
    const VALUE: Self::Type;
}

/// Alias for [`Const::Type`].
pub type TypeOf<C> = <C as Const>::Type;

/// Alias for [`Const::VALUE`]. Prefer this over accessing `VALUE` directly.
///
/// Using the associated constant through this function rather than directly causes it to only be
/// evaluated when the branch that it is used in is actually executed, assuming that the execution
/// happens at compile time (i.e. this does not apply to usage in regular `fn`s).
/// This means that it may improve compile times, avoid errors for recursive consts and avoid evaluating
/// panics.
///
/// For example:
/// ```
/// # use generic_upper_bound::*;
/// struct Fallible;
/// impl Const for Fallible {
///     type Type = ();
///     const VALUE: Self::Type = panic!();
/// }
/// const _: () = if false { const_value::<Fallible>() };  // this compiles
/// ```
/// ```compile_fail
/// # use generic_upper_bound::*;
/// # struct Fallible;
/// # impl Const for Fallible {
/// #     type Type = ();
/// #     const VALUE: Self::Type = panic!();
/// # }
/// const _: () = if false { Fallible::VALUE }; // this gives a compile error
/// ```
pub const fn const_value<C: Const + ?Sized>() -> C::Type {
    C::VALUE
}

/// Allows performing an evaluation of a [`Const`] after converting an associated const `usize` to a
/// best-effort upper bound `const ...: usize`.
pub trait AcceptUpperBound {
    /// The output type of the evaluation.
    type Output;

    /// The desired value and lower bound that the implementor wants to be passed to [`Self::Eval`].
    const DESIRED_GENERIC: usize;

    /// Evals the constant by mapping a generic parameter that is at least the desired value
    /// to the output value. `const_value::<Eval<N>>()` should be indistinguishable for
    /// all `UPPER_BOUND >= DESIRED` passed to this.
    type Eval<const UPPER: usize>: Const<Type = Self::Output>;
}

struct Impl<F>(F);

mod implementation;

/// Returns [`F::DESIRED_GENERIC`](AcceptUpperBound::DESIRED_GENERIC).
pub const fn desired_generic<F: AcceptUpperBound>() -> usize {
    Impl::<F>::DESIRED
}

/// Returns the parameter that [`eval_with_upper_bound`] passes to [`F::Eval`](AcceptUpperBound::Eval).
pub const fn get_upper_bound<F: AcceptUpperBound>() -> usize {
    Impl::<F>::ACTUAL
}

/// Evaluates [`AcceptUpperBound`].
///
/// In the language of `generic_const_exprs`, this function returns
/// `const_value::<F::Eval<{ get_upper_bound::<F>() }>>()`
pub const fn eval_with_upper_bound<F: AcceptUpperBound>() -> F::Output {
    Impl::<F>::EVAL
}
