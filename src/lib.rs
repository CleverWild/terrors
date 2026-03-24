#![cfg_attr(
    feature = "nightly",
    feature(error_generic_member_access, specialization),
    allow(incomplete_features)
)]
#![no_std]
#![cfg_attr(doc, doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")))]
#![allow(clippy::type_complexity)]

#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#[cfg(doctest)]
pub struct ReadmeDoctests;

#[macro_use]
mod utils;

mod broaden;
mod enums;

mod one_of;
mod subset;
mod type_set;

pub use broaden::Broaden;
pub use enums::*;
pub use one_of::OneOf;
pub use subset::SubsetErr;
pub use type_set::{Contains, DrainInto, EnumRuntime, Narrow, SupersetOf, TupleForm, TypeSet};

/* ------------------------- Helpers ----------------------- */

/// The terminal element of a type-level `Cons` list.
///
/// Serves as the base case for all recursive trait implementations over
/// the [`Cons`] heterogeneous list. `End` is uninhabited (like `!`) — it
/// can never be constructed at runtime, so any match on it is unreachable.
#[doc(hidden)]
pub type End = core::convert::Infallible;

/// A single node in a compile-time heterogeneous linked list.
///
/// The full type-level list for a tuple `(T1, T2, T3)` is represented as
/// `Cons<T1, Cons<T2, Cons<T3, End>>>`. This structure is stored in
/// [`TypeSet::Variants`] and is used by traits like [`Contains`], [`Narrow`],
/// and [`SupersetOf`] to perform compile-time set arithmetic.
///
/// `Cons` is never constructed at runtime; it exists purely as a type-level
/// token. Matching on it is always `unreachable!`.
///
/// [`TypeSet::Variants`]: crate::TypeSet::Variants
/// [`Contains`]: crate::Contains
/// [`Narrow`]: crate::Narrow
/// [`SupersetOf`]: crate::SupersetOf
#[doc(hidden)]
#[derive(Debug)]
pub struct Cons<Head, Tail>(core::marker::PhantomData<Head>, Tail);

/// A phantom wrapper used to encode the *recursive step* of a trait search.
///
/// When a trait like [`Narrow`] or [`SupersetOf`] must prove that some target
/// type lives in the *tail* of a [`Cons`] list, the compiler needs to
/// distinguish the "found at head" case (`Index = End`) from the "recurse into
/// tail" case (`Index = Recurse<…>`). `Recurse<Tail>` carries that tail-index
/// path and is never constructed at runtime.
///
/// [`Narrow`]: crate::Narrow
/// [`SupersetOf`]: crate::SupersetOf
#[doc(hidden)]
#[derive(Debug)]
pub struct Recurse<Tail>(Tail);
