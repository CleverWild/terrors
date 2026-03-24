//! Similar to anonymous unions / enums in languages that support type narrowing.

use core::{error::Error, fmt, ops::Deref};

#[cfg(feature = "nightly")]
use crate::type_set::TryCastNestedInto;
use crate::{
    Cons, End,
    type_set::{Contains, DrainInto, EnumRuntime, Narrow, TupleForm, TypeSet},
};

/* ------------------------- OneOf ----------------------- */

/// `OneOf` is an open sum type like a union or sum type in other languages.
///
/// It differs from an enum
/// in that you do not need to define any actual new type
/// in order to hold some specific combination of variants,
/// but rather you simply describe the `OneOf` as holding
/// one value out of several specific possibilities,
/// defined by using a tuple of those possible variants
/// as the generic parameter for the `OneOf`.
///
/// For example, a `OneOf<( &'static str, u32)>` contains either
/// a ` &'static str` or a `u32`. The value over a simple `Result`
/// or other traditional enum starts to become apparent in larger
/// codebases where error handling needs to occur in
/// different places for different errors. `OneOf` allows
/// you to quickly specify a function's return value as
/// involving a precise subset of errors that the caller
/// can clearly reason about.
pub struct OneOf<E: TypeSet> {
    pub(crate) value: E::Enum,
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync + Error>() {}
    assert_send_sync::<OneOf<()>>();
};

impl<T1> Deref for OneOf<(T1,)>
where
    T1: 'static,
{
    type Target = T1;

    #[inline]
    fn deref(&self) -> &T1 {
        match &self.value {
            crate::E1::T1(value) => value,
        }
    }
}

impl<T1> From<T1> for OneOf<(T1,)>
where
    T1: 'static,
{
    #[inline]
    fn from(t: T1) -> Self {
        Self::new(t)
    }
}

impl<E> Clone for OneOf<E>
where
    E: TypeSet,
    E::Enum: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}
impl<E> fmt::Debug for OneOf<E>
where
    E: TypeSet,
    E::Enum: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}

impl<E> fmt::Display for OneOf<E>
where
    E: TypeSet,
    E::Enum: fmt::Display,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl<E> Error for OneOf<E>
where
    E: TypeSet,
    E::Enum: Error,
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.value.source()
    }
}

impl<E> OneOf<E>
where
    E: TypeSet,
{
    /// Create a new `OneOf`.
    #[inline]
    pub fn new<T, Index>(t: T) -> Self
    where
        T: 'static,
        E: EnumRuntime,
        E::Variants: Contains<T, Index>,
    {
        let result = E::from_owned(t);
        #[cfg(feature = "nightly")]
        let result = result.map_err(<T as TryCastNestedInto<E>>::try_cast_nested_into);
        result.map_or_else(
            |_| {
                unreachable!(
                    "`Contains<T, _>` guarantees T is part of E and construction cannot fail"
                )
            },
            |value| Self { value },
        )
    }

    /// Attempt to downcast the `OneOf` into a specific type, and
    /// if that fails, return a `OneOf` which does not contain that
    /// type as one of its possible variants.
    #[inline]
    pub fn narrow<Target, Index>(
        self,
    ) -> Result<
        Target,
        OneOf<<<E::Variants as Narrow<Target, Index>>::Remainder as TupleForm>::Tuple>,
    >
    where
        Target: 'static,
        E: EnumRuntime,
        E::Variants: Narrow<Target, Index>,
        <<E::Variants as Narrow<Target, Index>>::Remainder as TupleForm>::Tuple: EnumRuntime,
    {
        E::narrow_type::<Target>(self.value).map_err(|e| {
            let Ok(remainder_value) =
                E::try_cast::<<<<E as TypeSet>::Variants as Narrow<Target, Index>>::Remainder as TupleForm>::Tuple>(e)
            else {
                unreachable!(
                    "Cast to narrowed remainder type should never fail since `Target` is not the variant type"
                );
            };

            OneOf {
                value: remainder_value,
            }
        })
    }

    /// For a `OneOf` with a single variant, return the contained value.
    #[inline]
    pub fn take<Target>(self) -> Target
    where
        Target: 'static,
        E: TypeSet<Variants = Cons<Target, End>> + EnumRuntime,
    {
        let Ok(target) = E::narrow_type(self.value) else {
            unreachable!("A single-variant OneOf must hold the only possible type")
        };
        target
    }

    /// Consumes the [`Self`] and converts whichever variant it holds into `O`,
    /// requiring every possible variant to implement `Into<O>`.
    ///
    /// We keep this as an inherent method instead of implementing `Into<O> for OneOf<E>`
    /// because that impl conflicts with `core`'s blanket `Into` implementation and
    /// is rejected by coherence rules.
    #[inline]
    pub fn into<O>(self) -> O
    where
        E: DrainInto<O>,
    {
        <E as DrainInto<O>>::drain(self)
    }

    /// Convert the `OneOf` to an owned enum for
    /// use in pattern matching etc...
    #[inline]
    pub fn to_enum(self) -> E::Enum {
        self.value
    }

    /// Borrow the enum as an enum for use in
    /// pattern matching etc...
    #[inline]
    pub fn as_enum<'a>(&'a self) -> E::EnumRef<'a>
    where
        E::EnumRef<'a>: From<&'a Self>,
    {
        E::EnumRef::from(self)
    }
}
