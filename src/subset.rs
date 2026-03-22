use crate::{EnumRuntime, OneOf, SupersetOf, TupleForm, TypeSet};

/// Splits [Self] by checking whether the held error belongs to a requested subset O.
pub trait SubsetErr<E: TypeSet> {
    /// Output type after subsetting to O.
    type Output<O, Index>: Sized
    where
        E::Variants: SupersetOf<O::Variants, Index>,
        O: TypeSet + EnumRuntime,
        <<E::Variants as SupersetOf<O::Variants, Index>>::Remainder as TupleForm>::Tuple:
            EnumRuntime;

    /// Designed to be used in a method chain before ? to narrow down
    /// the error type to a subset of variants that the caller can handle.
    fn subset<O, Index>(self) -> Self::Output<O, Index>
    where
        E: EnumRuntime,
        O: TypeSet + EnumRuntime,
        E::Variants: SupersetOf<O::Variants, Index>,
        <<E::Variants as SupersetOf<O::Variants, Index>>::Remainder as TupleForm>::Tuple:
            EnumRuntime;
}

impl<E: TypeSet> SubsetErr<E> for OneOf<E> {
    type Output<O, Index>
        = Result<
        OneOf<O>,
        OneOf<<<E::Variants as SupersetOf<O::Variants, Index>>::Remainder as TupleForm>::Tuple>,
    >
    where
        E::Variants: SupersetOf<O::Variants, Index>,
        O: TypeSet + EnumRuntime,
        <<E::Variants as SupersetOf<O::Variants, Index>>::Remainder as TupleForm>::Tuple:
            EnumRuntime;

    /// Attempt to split a subset of variants out of the `OneOf`,
    /// returning the remainder of possible variants if the value
    /// does not have one of the Target List types.
    #[inline]
    fn subset<O, Index>(self) -> Self::Output<O, Index>
    where
        E: EnumRuntime,
        O: TypeSet + EnumRuntime,
        E::Variants: SupersetOf<O::Variants, Index>,
        <<E::Variants as SupersetOf<O::Variants, Index>>::Remainder as TupleForm>::Tuple:
            EnumRuntime,
    {
        match <E as EnumRuntime>::try_cast::<O>(self.value) {
            Ok(o) => Ok(OneOf { value: o }),
            Err(e) => {
                type Remainder<E, O, Index> = <<<E as TypeSet>::Variants as SupersetOf<
                    <O as TypeSet>::Variants,
                    Index,
                >>::Remainder as TupleForm>::Tuple;
                let Ok(remainder_val) = <E as EnumRuntime>::try_cast::<Remainder<E, O, Index>>(e)
                else {
                    unreachable!("Value not in subset must be in remainder")
                };
                Err(OneOf {
                    value: remainder_val,
                })
            }
        }
    }
}

impl<T, E: TypeSet> SubsetErr<E> for Result<T, OneOf<E>> {
    type Output<O, Index>
        = Result<
        Result<
            T,
            OneOf<<<E::Variants as SupersetOf<O::Variants, Index>>::Remainder as TupleForm>::Tuple>,
        >,
        OneOf<O>,
    >
    where
        E::Variants: SupersetOf<O::Variants, Index>,
        O: TypeSet + EnumRuntime,
        <<E::Variants as SupersetOf<O::Variants, Index>>::Remainder as TupleForm>::Tuple:
            EnumRuntime;

    /// This method splits a `Result<T, OneOf<E>>` into three outcomes:
    ///
    /// - `Ok(Ok(T))` when the original result was successful.
    /// - `Err(OneOf<O>)` when the error belongs to subset `O`, so it can be propagated with `?`.
    /// - `Ok(Err(OneOf<Rest>))` when the error is not `O`, preserving the remainder.
    #[inline]
    fn subset<O, Index>(self) -> Self::Output<O, Index>
    where
        E: EnumRuntime,
        O: TypeSet + EnumRuntime,
        E::Variants: SupersetOf<O::Variants, Index>,
        <<E::Variants as SupersetOf<O::Variants, Index>>::Remainder as TupleForm>::Tuple:
            EnumRuntime,
    {
        match self {
            Ok(value) => Ok(Ok(value)),
            Err(errs) => match errs.subset::<O, _>() {
                Ok(o) => Err(o),
                Err(rest) => Ok(Err(rest)),
            },
        }
    }
}
