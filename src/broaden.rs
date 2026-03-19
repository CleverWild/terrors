use crate::{type_set::Contains, EnumRuntime, OneOf, SupersetOf, TypeSet};

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
    fn broaden<Other, Index>(self) -> Self::Output<Other>
    where
        E: EnumRuntime,
        Other: TypeSet + EnumRuntime,
        Other::Variants: SupersetOf<E::Variants, Index>;
}

impl<E: TypeSet> Broaden<E> for OneOf<E> {
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

impl<E: TypeSet> Broaden<E> for Option<OneOf<E>> {
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

impl<T, E: TypeSet> Broaden<E> for Result<T, OneOf<E>> {
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

/// Extension trait on [`Result`] providing a shortcut for `.map_err(|e| OneOf::new(e))`.
///
/// Converts `Result<T, E>` into `Result<T, OneOf<O>>` where `O` contains `E`
/// as one of its variants. Useful when a function returning a plain error type
/// needs to be composed with functions returning [`OneOf`]-based errors.
///
/// # Example
///
/// ```rust
/// use terrors::{BroadErr, OneOf};
///
/// fn parse(s: &str) -> Result<i32, std::num::ParseIntError> {
///     s.parse()
/// }
///
/// fn run() -> Result<i32, OneOf<(std::io::Error, std::num::ParseIntError)>> {
///     let n = parse("42").broad_err()?;
///     Ok(n)
/// }
/// ```
pub trait BroadErr<T, E: 'static> {
    /// Converts the error variant into a [`OneOf<O>`] by wrapping it.
    ///
    /// The compiler verifies at compile time that `E` is one of the variants of `O`.
    fn broad_err<Other, Index>(self) -> Result<T, OneOf<Other>>
    where
        Other: TypeSet + EnumRuntime,
        Other::Variants: Contains<E, Index>;
}

impl<T, E: 'static> BroadErr<T, E> for Result<T, E> {
    fn broad_err<Other, Index>(self) -> Result<T, OneOf<Other>>
    where
        Other: TypeSet + EnumRuntime,
        Other::Variants: Contains<E, Index>,
    {
        self.map_err(|e| {
            let boxed: Box<dyn core::any::Any> = Box::new(e);
            OneOf {
                value: Other::enum_from_any(boxed),
            }
        })
    }
}
