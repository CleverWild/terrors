use crate::{EnumRuntime, OneOf, SupersetOf, TypeSet};

pub trait BroadenErr<E: TypeSet> {
    type Output<O: TypeSet>: Sized;

    /// Turns the `OneOf` into a `OneOf` with a set of variants
    /// which is a superset of the current one. This may also be
    /// the same set of variants, but in a different order.
    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>;
}

impl<E: TypeSet> BroadenErr<E> for OneOf<E> {
    type Output<O: TypeSet> = OneOf<O>;

    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>,
    {
        let boxed = E::enum_into_any(self.value);
        OneOf {
            value: Other::enum_from_any(boxed),
        }
    }
}

impl<E: TypeSet> BroadenErr<E> for Option<OneOf<E>> {
    type Output<O: TypeSet> = Option<OneOf<O>>;

    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>,
    {
        self.map(OneOf::broaden)
    }
}

impl<T, E: TypeSet> BroadenErr<E> for Result<T, OneOf<E>> {
    type Output<O: TypeSet> = Result<T, OneOf<O>>;

    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>,
    {
        self.map_err(OneOf::broaden)
    }
}
