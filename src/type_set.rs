//! Type-level set inclusion and difference, inspired by frunk's approach: <https://archive.is/YwDMX>.

use crate::{Cons, End, OneOf, Recurse, enums::*};

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
            #[inline]
            fn type_id(e: &Self::Enum) -> core::any::TypeId {
                match e {
                    $(
                        $enum::$ty(_) => core::any::TypeId::of::<$ty>(),
                    )+
                }
            }
            
            #[inline]
            unsafe fn try_from_raw(id: core::any::TypeId, ptr: *mut ()) -> Option<Self::Enum> {
                $(
                    if id == core::any::TypeId::of::<$ty>() {
                        // SAFETY: Caller guarantees `ptr` points to a valid owned `$ty`
                        // when `id` matches this branch.
                        return Some($enum::$ty(unsafe { core::ptr::read(ptr.cast::<$ty>()) }));
                    }
                )+
                None
            }

            #[inline]
            fn try_cast<O: EnumRuntime>(e: Self::Enum) -> Result<O::Enum, Self::Enum> {
                match e {
                    $(
                        $enum::$ty(v) => match O::from_owned(v) {
                            Ok(o) => Ok(o),
                            Err(v) => Err($enum::$ty(v)),
                        }
                    )+
                }
            }

            #[inline]
            fn narrow_type<Target: 'static>(e: Self::Enum) -> Result<Target, Self::Enum> {
                if <Self as EnumRuntime>::type_id(&e) == core::any::TypeId::of::<Target>() {
                    match e {
                        $(
                            $enum::$ty(v) => {
                                let mut v = core::mem::ManuallyDrop::new(v);
                                let ptr = (&raw mut *v).cast::<Target>();
                                // SAFETY: We only enter this branch when `Target`'s `TypeId`
                                // matches the currently stored variant type (`$ty`), so `ptr`
                                // points to a valid initialized value of `Target`.
                                let val = unsafe { core::ptr::read(ptr) };
                                Ok(val)
                            }
                        )+
                    }
                } else {
                    Err(e)
                }
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

/// Runtime bridge for moving values between generated [`TypeSet::Enum`] forms
/// without dynamic allocation.
///
/// [`OneOf`] stores its value as a `TypeSet::Enum`, while set operations
/// (narrowing, broadening, subsetting) may need to transfer ownership between
/// different enum layouts. `EnumRuntime` provides the low-level primitive
/// [`try_from_raw`](EnumRuntime::try_from_raw) and safe helpers built on top.
///
/// Automatically implemented for all tuples that implement [`TypeSet`].
/// Do not implement manually.
pub trait EnumRuntime: TypeSet + Sized {
    /// The `TypeId` of the currently held variant.
    fn type_id(e: &Self::Enum) -> core::any::TypeId;

    /// Attempts to move an owned value `T` into `Self::Enum`.
    ///
    /// Returns `Ok(Self::Enum)` when `T` is one of this set variants,
    /// otherwise returns the original value in `Err(T)`.
    #[inline]
    fn from_owned<T: 'static>(value: T) -> Result<Self::Enum, T> {
        let mut value = core::mem::ManuallyDrop::new(value);
        let id = core::any::TypeId::of::<T>();
        let ptr = (&raw mut *value).cast::<()>();

        // SAFETY: `ptr` points to a valid owned `T` for the duration of this call.
        // If `try_from_raw` succeeds, ownership moves into the returned enum.
        // If it fails, we reconstruct and return the original value.
        let maybe = unsafe { Self::try_from_raw(id, ptr) };
        let Some(e) = maybe else {
            return Err(core::mem::ManuallyDrop::into_inner(value));
        };
        Ok(e)
    }

    /// Attempts to construct `Self::Enum` by reading from `ptr` if `type_id` matches one of the variants.
    ///
    /// # Safety
    /// Caller must ensure that if `type_id` matches one of the variants, `ptr` points to a valid
    /// owned instance of that type in memory. The caller must then `forget` the original instance
    /// if this returns `Some`, as ownership has been transferred to the newly constructed enum.
    unsafe fn try_from_raw(id: core::any::TypeId, ptr: *mut ()) -> Option<Self::Enum>;

    /// Try to cast this enum into another Enum type. If successful, returns the new enum.
    /// If unsuccessful (the variant type is not in `O`), returns `Err(self)`.
    fn try_cast<O: EnumRuntime>(e: Self::Enum) -> Result<O::Enum, Self::Enum>;

    /// Reads the held value safely if its type matches `T`.
    /// Returns the unwrapped value on success, or the original enum if the type differs.
    fn narrow_type<T: 'static>(e: Self::Enum) -> Result<T, Self::Enum>;
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
    #[inline]
    fn type_id(e: &Self::Enum) -> core::any::TypeId {
        match *e {}
    }

    #[inline]
    unsafe fn try_from_raw(_: core::any::TypeId, _: *mut ()) -> Option<Self::Enum> {
        None
    }

    #[inline]
    fn try_cast<O: EnumRuntime>(e: Self::Enum) -> Result<O::Enum, Self::Enum> {
        match e {}
    }

    #[inline]
    fn narrow_type<T: 'static>(e: Self::Enum) -> Result<T, Self::Enum> {
        match e {}
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
    Self: TupleForm,
{
    type Remainder = Tail;
}

/// Recursive case where the search Target is in the Tail of the Variants.
impl<Head, Tail, Target, Index> Narrow<Target, Recurse<Index>> for Cons<Head, Tail>
where
    Tail: TupleForm + Narrow<Target, Index>,
    Self: TupleForm,
    Cons<Head, <Tail as Narrow<Target, Index>>::Remainder>: TupleForm,
{
    type Remainder = Cons<Head, <Tail as Narrow<Target, Index>>::Remainder>;
}

const _: () = {
    const fn can_narrow<Types, Target, Remainder, Index>()
    where
        Types: Narrow<Target, Index, Remainder = Remainder>,
    {
    }

    type T0 = <(u32, &'static str) as TypeSet>::Variants;

    can_narrow::<T0, u32, _, _>();
    can_narrow::<T0, &'static str, Cons<u32, End>, _>();
};

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
/// let e: OneOf<(u32, u8)> = OneOf::new(5_u8);
///
/// let s: u64 = e.into();
/// assert_eq!(s, 5);
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
            #[inline]
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
            #[inline]
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

/// Base case.
impl<T: TupleForm> SupersetOf<End, End> for T {
    type Remainder = T;
}

/// Recursive case - more complex because we have to reason about the Index itself as a
/// heterogenous list.
impl<SubHead, SubTail, SuperHead, SuperTail, HeadIndex, TailIndex>
    SupersetOf<Cons<SubHead, SubTail>, Cons<HeadIndex, TailIndex>> for Cons<SuperHead, SuperTail>
where
    Self: Narrow<SubHead, HeadIndex>,
    <Self as Narrow<SubHead, HeadIndex>>::Remainder: SupersetOf<SubTail, TailIndex>,
{
    type Remainder = <<Self as Narrow<SubHead, HeadIndex>>::Remainder as SupersetOf<
        SubTail,
        TailIndex,
    >>::Remainder;
}

const _: () = {
    const fn is_superset<S1, S2, Remainder, Index>()
    where
        S1: SupersetOf<S2, Index, Remainder = Remainder>,
    {
    }

    type T0 = <(u32,) as TypeSet>::Variants;
    type T1A = <(u32, &'static str) as TypeSet>::Variants;
    type T1B = <(&'static str, u32) as TypeSet>::Variants;
    type T2 = <(&'static str, i32, u32) as TypeSet>::Variants;
    type T3 = <((), (), u32, f32, &'static str, f64, i32) as TypeSet>::Variants;

    is_superset::<T0, T0, _, _>();
    is_superset::<T1A, T1A, _, _>();
    is_superset::<T1A, T1B, _, _>();
    is_superset::<T1B, T1A, _, _>();
    is_superset::<T2, T2, _, _>();
    is_superset::<T1A, T0, _, _>();
    is_superset::<T1B, T0, _, _>();
    is_superset::<T2, T0, <(&'static str, i32) as TypeSet>::Variants, _>();
    is_superset::<T2, T1A, <(i32,) as TypeSet>::Variants, _>();
    is_superset::<T2, T1B, <(i32,) as TypeSet>::Variants, _>();
    is_superset::<T3, T1A, <((), (), f32, f64, i32) as TypeSet>::Variants, _>();
    is_superset::<T3, T1B, _, _>();
    is_superset::<T3, T0, _, _>();
    is_superset::<T3, T2, _, _>();

    type T5sup = <(u8, u16, u32, u64, u128) as TypeSet>::Variants;
    type T5sub = <(u8, u128) as TypeSet>::Variants;
    type T5rem = <(u16, u32, u64) as TypeSet>::Variants;

    is_superset::<T5sup, T5sub, T5rem, _>();
};
