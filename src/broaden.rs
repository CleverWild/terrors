#[cfg(not(feature = "nightly"))]
use crate::SupersetOf;
use crate::{EnumRuntime, OneOf, TypeSet};

/// Broadens a [`OneOf`] (or containers that hold it) into a superset of variants.
///
/// This is mainly ergonomic sugar so you can write `.map_err(OneOf::broaden)` and
/// similar method chains while preserving compile-time subset/superset checks.
pub trait Broaden<E: TypeSet> {
    /// Resulting container type after broadening to `O`.
    type Output<O: TypeSet>: Sized;

    /// Turns the `OneOf` into a `OneOf` with a set of variants
    /// which is a superset of the current one. This may also be
    /// the same set of variants, but in a different order.
    #[cfg(not(feature = "nightly"))]
    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>;

    /// Turns the `OneOf` into another `OneOf` in nightly mode,
    /// allowing flatten-aware conversions through runtime cast paths.
    #[cfg(feature = "nightly")]
    fn broaden<Other>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime;
}

impl<E: TypeSet> Broaden<E> for OneOf<E> {
    type Output<O: TypeSet> = OneOf<O>;

    #[cfg(not(feature = "nightly"))]
    #[inline]
    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>,
    {
        let Ok(value) = E::try_cast::<Other>(self.value) else {
            unreachable!("Cast to broadened superset should never fail")
        };

        OneOf { value }
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn broaden<Other>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
    {
        let Ok(value) = E::try_cast::<Other>(self.value) else {
            unreachable!(
                "Cast to broadened target should never fail in nightly when used with compatible layouts"
            )
        };

        OneOf { value }
    }
}

impl<E: TypeSet> Broaden<E> for Option<OneOf<E>> {
    type Output<O: TypeSet> = Option<OneOf<O>>;

    #[cfg(not(feature = "nightly"))]
    #[inline]
    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>,
    {
        self.map(OneOf::broaden::<Other, Index>)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn broaden<Other>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
    {
        self.map(OneOf::broaden::<Other>)
    }
}

impl<T, E: TypeSet> Broaden<E> for Result<T, OneOf<E>> {
    type Output<O: TypeSet> = Result<T, OneOf<O>>;

    #[cfg(not(feature = "nightly"))]
    #[inline]
    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>,
    {
        self.map_err(OneOf::broaden::<Other, Index>)
    }

    #[cfg(feature = "nightly")]
    #[inline]
    fn broaden<Other>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
    {
        self.map_err(OneOf::broaden::<Other>)
    }
}
