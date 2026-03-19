#![cfg_attr(
    feature = "error_provide_feature",
    feature(error_generic_member_access)
)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

macro_rules! with_supported_type_sets {
    ($callback:ident) => {
        with_supported_type_sets! {
            @build $callback;
            prefix: [];
            each: [];
            enums: E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13, E14, E15, E16;
            types: T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16;
        }
    };
    (@build $callback:ident;
        prefix: [$($prefix:ident,)*];
        each: [$($built:tt)*];
        enums: $enum:ident;
        types: $next:ident;
    ) => {
        $callback! {
            each:
                $($built)*
                $enum => $($prefix,)* $next;
        }
    };
    (@build $callback:ident;
        prefix: [$($prefix:ident,)*];
        each: [$($built:tt)*];
        enums: $enum:ident, $($rest_enums:ident),+;
        types: $next:ident, $($rest_types:ident),+;
    ) => {
        with_supported_type_sets! {
            @build $callback;
            prefix: [$($prefix,)* $next,];
            each: [
                $($built)*
                $enum => $($prefix,)* $next;
            ];
            enums: $($rest_enums),+;
            types: $($rest_types),+;
        }
    };
}

pub(crate) use with_supported_type_sets;

#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#[cfg(doctest)]
pub struct ReadmeDoctests;

mod broaden;
mod enums;
mod fold;
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
#[derive(Debug)]
pub enum End {}

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
