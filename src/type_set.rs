//! Type-level set inclusion and difference, inspired by frunk's approach: <https://archive.is/YwDMX>
use core::any::Any;

use crate::{enums::*, Cons, End, OneOf, Recurse};

macro_rules! enum_from_any_branch {
    ($any:ident, $enum:ident, $ty:ident) => {
        $enum::$ty(*$any.downcast().unwrap())
    };
    ($any:ident, $enum:ident, $ty:ident, $($rest_ty:ident),+) => {
        if $any.is::<$ty>() {
            $enum::$ty(*$any.downcast().unwrap())
        } else {
            enum_from_any_branch!($any, $enum, $($rest_ty),+)
        }
    };
}

macro_rules! tuple_type {
    ($only:ident) => {
        ($only,)
    };
    ($head:ident, $($tail:ident),+ $(,)?) => {
        ($head, $($tail),+)
    };
}

macro_rules! cons_type {
    ($head:ident $(, $tail:ident)* $(,)?) => {
        Cons<$head, cons_type!($($tail),*)>
    };
    () => {
        End
    };
}

macro_rules! impl_tuple_type_set {
    ($enum:ident; $($ty:ident),+ $(,)?) => {
        impl<$($ty),+> TypeSet for tuple_type!($($ty),+) {
            type Variants = cons_type!($($ty),+);
            type Enum = $enum<$($ty),+>;
            type EnumRef<'a> = $enum<$( &'a $ty ),+> where Self: 'a;
        }

        impl<$($ty: 'static),+> EnumRuntime for tuple_type!($($ty),+) {
            fn enum_into_any(e: Self::Enum) -> Box<dyn Any> {
                match e {
                    $(
                        $enum::$ty(value) => Box::new(value),
                    )+
                }
            }

            fn enum_ref_as_any(e: &Self::Enum) -> &dyn Any {
                match e {
                    $(
                        $enum::$ty(value) => value as &dyn Any,
                    )+
                }
            }

            fn enum_from_any(any: Box<dyn Any>) -> Self::Enum {
                enum_from_any_branch!(any, $enum, $($ty),+)
            }
        }

        impl<$($ty),+> TupleForm for cons_type!($($ty),+) {
            type Tuple = tuple_type!($($ty),+);
        }
    };
}

/* ------------------------- TypeSet implemented for tuples ----------------------- */

/// Maps a tuple of distinct types to the internal representation used by [`OneOf`].
///
/// This is the core bridge between user-facing tuple syntax (e.g. `(IoError, ParseError)`)
/// and the machinery underneath:
///
/// - [`Variants`](TypeSet::Variants) — a `Cons<T1, Cons<T2, … End>>` linked list enabling
///   compile-time set arithmetic ([`Contains`], [`Narrow`], [`SupersetOf`]).
/// - [`Enum`](TypeSet::Enum) — the anonymous generated enum (e.g. `E2<T1, T2>`) that stores
///   the value at runtime inside a [`OneOf`].
/// - [`EnumRef`](TypeSet::EnumRef) — the borrowed version of `Enum`, returned by
///   [`OneOf::as_enum`].
///
/// Implementations are generated automatically for tuples of 0–16 types.
/// You never implement this trait manually.
pub trait TypeSet {
    /// Type-level linked-list form (`Cons<..., End>`) of this tuple set.
    type Variants: TupleForm;

    /// Lifted runtime enum used as storage for [`OneOf<Self>`](crate::OneOf).
    type Enum;

    /// Borrowed lifted runtime enum, typically produced by [`OneOf::as_enum`](crate::OneOf::as_enum).
    type EnumRef<'a>
    where
        Self: 'a;
}

/// Runtime bridge between a [`TypeSet::Enum`] and a type-erased `Box<dyn Any>`.
///
/// [`OneOf`] stores its value as a `TypeSet::Enum`, but set-arithmetic operations
/// (narrowing, broadening, subsetting) need to pass the inner value around without
/// knowing its concrete type. `EnumRuntime` provides three conversions:
///
/// - [`enum_into_any`](EnumRuntime::enum_into_any) — consumes the enum, boxing its inner value.
/// - [`enum_ref_as_any`](EnumRuntime::enum_ref_as_any) — borrows the inner value as `&dyn Any`.
/// - [`enum_from_any`](EnumRuntime::enum_from_any) — reconstructs the enum from a `Box<dyn Any>`;
///   panics if the boxed type does not match any variant.
///
/// Automatically implemented for all tuples that implement [`TypeSet`].
/// Do not implement manually.
pub trait EnumRuntime: TypeSet {
    /// Consumes a lifted enum and erases the held variant value to `Box<dyn Any>`.
    fn enum_into_any(e: Self::Enum) -> Box<dyn Any>;

    /// Borrows the inner variant value as `&dyn Any` without moving it.
    fn enum_ref_as_any(e: &Self::Enum) -> &dyn Any;

    /// Rebuilds a lifted enum from `Box<dyn Any>`.
    ///
    /// # Panics
    ///
    /// Implementations may panic if `any` does not contain one of the set's variant types.
    fn enum_from_any(any: Box<dyn Any>) -> Self::Enum;
}

impl TypeSet for () {
    type Variants = End;
    type Enum = crate::E0;
    type EnumRef<'a>
        = crate::E0
    where
        Self: 'a;
}

impl EnumRuntime for () {
    fn enum_into_any(e: Self::Enum) -> Box<dyn Any> {
        match e {}
    }

    fn enum_ref_as_any(e: &Self::Enum) -> &dyn Any {
        match *e {}
    }

    fn enum_from_any(_: Box<dyn Any>) -> Self::Enum {
        unreachable!("cannot build E0 from Box<dyn Any>");
    }
}

macro_rules! impl_supported_tuple_type_sets {
    (each: $($enum:ident => $($ty:ident),+;)+) => {
        $(
            impl_tuple_type_set!($enum; $($ty),+);
        )+
    };
}

crate::with_supported_type_sets!(impl_supported_tuple_type_sets);

/* ------------------------- TupleForm implemented for TypeSet ----------------------- */

/// The inverse of [`TypeSet`]: converts a type-level `Cons`/`End` list back to its tuple form.
///
/// Used internally after operations like [`Narrow`] and [`SupersetOf`] that produce a
/// modified `Cons` list — the result must be converted back to a tuple in order to
/// parametrize a new [`OneOf`].
///
/// Implementations are generated automatically alongside [`TypeSet`].
/// You never implement this trait manually.
pub trait TupleForm {
    /// Tuple representation corresponding to this `Cons`/`End` chain.
    type Tuple: TypeSet;
}

impl TupleForm for End {
    type Tuple = ();
}

/* ------------------------- Contains ----------------------- */

/// Compile-time proof that type `T` is a member of the `Cons` list `Self`.
///
/// The `Index` phantom parameter encodes the *position* of `T` in the list as a
/// `End` (found at the head) or `Cons<Index, ()>` (found further in the tail) path,
/// allowing the compiler to select the correct impl without ambiguity.
///
/// You never call or implement this trait directly. It appears as a bound inside
/// [`OneOf::new`] to guarantee at compile time that the value being wrapped is one
/// of the declared variants.
pub trait Contains<T, Index> {}

/// Base case implementation for when the Cons Head is T.
impl<T, Tail> Contains<T, End> for Cons<T, Tail> {}

/// Recursive case for when the Cons Tail contains T.
impl<T, Index, Head, Tail> Contains<T, Cons<Index, ()>> for Cons<Head, Tail> where
    Tail: Contains<T, Index>
{
}

/* ------------------------- Narrow ----------------------- */

/// Compile-time extraction of a single type `Target` from a `Cons` list.
///
/// Produces `Remainder` — the original list with `Target` removed — so that a failed
/// [`OneOf::narrow`] call can return a `OneOf` over the remaining variants.
///
/// The `Index` phantom parameter encodes the position of `Target` within the list.
/// Two implementations cover all cases:
/// - base case: `Target` is at the head (`Index = End`).
/// - recursive case: `Target` is somewhere in the tail (`Index = Recurse<…>`).
pub trait Narrow<Target, Index>: TupleForm {
    /// `Self` with `Target` removed.
    type Remainder: TupleForm;
}

/// Base case where the search Target is in the Head of the Variants.
impl<Target, Tail> Narrow<Target, End> for Cons<Target, Tail>
where
    Tail: TupleForm,
    Cons<Target, Tail>: TupleForm,
{
    type Remainder = Tail;
}

/// Recursive case where the search Target is in the Tail of the Variants.
impl<Head, Tail, Target, Index> Narrow<Target, Recurse<Index>> for Cons<Head, Tail>
where
    Tail: Narrow<Target, Index>,
    Tail: TupleForm,
    Cons<Head, Tail>: TupleForm,
    Cons<Head, <Tail as Narrow<Target, Index>>::Remainder>: TupleForm,
{
    type Remainder = Cons<Head, <Tail as Narrow<Target, Index>>::Remainder>;
}

fn _narrow_test() {
    fn can_narrow<Types, Target, Remainder, Index>()
    where
        Types: Narrow<Target, Index, Remainder = Remainder>,
    {
    }

    type T0 = <(u32, String) as TypeSet>::Variants;

    can_narrow::<T0, u32, _, _>();
    can_narrow::<T0, String, Cons<u32, End>, _>();
}

/* ------------------------- DrainInto ----------------------- */

/// Exhaustively converts a [`OneOf`] into a single output type `O` by consuming it.
///
/// Every variant in the set must implement `Into<O>`. The compiler proves this
/// statically, so no runtime pattern-matching or fallibility is involved.
///
/// # Example
///
/// ```rust
/// use terrors::OneOf;
///
/// let e: OneOf<(String, &str)> = OneOf::new("hello");
/// let s: String = e.into();
/// assert_eq!(s, "hello");
/// ```
pub trait DrainInto<O>: TypeSet + Sized {
    /// Consumes `e` and converts whichever variant it holds into `O`.
    fn drain(e: OneOf<Self>) -> O;
}

macro_rules! impl_drain_into {
    ($head:ident) => {
        impl<$head, O> DrainInto<O> for ($head,)
        where
            $head: Into<O> + 'static,
        {
            fn drain(e: OneOf<($head,)>) -> O {
                e.take().into()
            }
        }
    };
    ($head:ident, $($tail:ident),+) => {
        impl_drain_into!($($tail),+);
        impl<$head, $($tail),+, O> DrainInto<O> for ($head, $($tail),+)
        where
            $head: Into<O> + 'static,
            $($tail: 'static,)+
            ($($tail,)+): DrainInto<O>,
        {
            fn drain(e: OneOf<($head, $($tail),+)>) -> O {
                match e.narrow::<$head, _>() {
                    Ok(h) => h.into(),
                    Err(rest) => <($($tail,)+)>::drain(rest),
                }
            }
        }
    };
}

// Peel entries until the last one, which contains the full type list.
// impl_drain_into! is recursive and generates impls for all sub-lengths itself.
macro_rules! impl_supported_drain_into {
    (each: $enum:ident => $($ty:ident),+;) => {
        impl_drain_into!($($ty),+);
    };
    (each: $enum:ident => $($ty:ident),+; $($rest:tt)+) => {
        impl_supported_drain_into!(each: $($rest)+);
    };
}

crate::with_supported_type_sets!(impl_supported_drain_into);

/* ------------------------- SupersetOf ----------------------- */

/// Compile-time proof that every type in `Other` is also present in `Self`.
///
/// Used by [`OneOf::broaden`] (which requires `Other ⊇ Self`) and
/// [`OneOf::subset`] (which tests whether the held value belongs to a subset).
///
/// `Remainder` is the set of types that are in `Self` but *not* in `Other`. It
/// is used to construct the fallback [`OneOf`] returned by [`OneOf::subset`]
/// when the value does not belong to the target subset.
pub trait SupersetOf<Other, Index> {
    /// Types present in `Self` but not in `Other`.
    type Remainder: TupleForm;
}

/// Base case
impl<T: TupleForm> SupersetOf<End, End> for T {
    type Remainder = T;
}

/// Recursive case - more complex because we have to reason about the Index itself as a
/// heterogenous list.
impl<SubHead, SubTail, SuperHead, SuperTail, HeadIndex, TailIndex>
    SupersetOf<Cons<SubHead, SubTail>, Cons<HeadIndex, TailIndex>> for Cons<SuperHead, SuperTail>
where
    Cons<SuperHead, SuperTail>: Narrow<SubHead, HeadIndex>,
    <Cons<SuperHead, SuperTail> as Narrow<SubHead, HeadIndex>>::Remainder:
        SupersetOf<SubTail, TailIndex>,
{
    type Remainder =
        <<Cons<SuperHead, SuperTail> as Narrow<SubHead, HeadIndex>>::Remainder as SupersetOf<
            SubTail,
            TailIndex,
        >>::Remainder;
}

fn _superset_test() {
    fn is_superset<S1, S2, Remainder, Index>()
    where
        S1: SupersetOf<S2, Index, Remainder = Remainder>,
    {
    }

    type T0 = <(u32,) as TypeSet>::Variants;
    type T1A = <(u32, String) as TypeSet>::Variants;
    type T1B = <(String, u32) as TypeSet>::Variants;
    type T2 = <(String, i32, u32) as TypeSet>::Variants;
    type T3 = <(Vec<u8>, Vec<i8>, u32, f32, String, f64, i32) as TypeSet>::Variants;

    is_superset::<T0, T0, _, _>();
    is_superset::<T1A, T1A, _, _>();
    is_superset::<T1A, T1B, _, _>();
    is_superset::<T1B, T1A, _, _>();
    is_superset::<T2, T2, _, _>();
    is_superset::<T1A, T0, _, _>();
    is_superset::<T1B, T0, _, _>();
    is_superset::<T2, T0, <(String, i32) as TypeSet>::Variants, _>();
    is_superset::<T2, T1A, <(i32,) as TypeSet>::Variants, _>();
    is_superset::<T2, T1B, <(i32,) as TypeSet>::Variants, _>();
    is_superset::<T3, T1A, <(Vec<u8>, Vec<i8>, f32, f64, i32) as TypeSet>::Variants, _>();
    is_superset::<T3, T1B, _, _>();
    is_superset::<T3, T0, _, _>();
    is_superset::<T3, T2, _, _>();

    type T5sup = <(u8, u16, u32, u64, u128) as TypeSet>::Variants;
    type T5sub = <(u8, u128) as TypeSet>::Variants;
    type T5rem = <(u16, u32, u64) as TypeSet>::Variants;

    is_superset::<T5sup, T5sub, T5rem, _>();
}
