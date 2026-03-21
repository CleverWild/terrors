//! Similar to anonymous unions / enums in languages that support type narrowing.

use core::{any::Any, fmt, ops::Deref};
use std::error::Error;

use crate::{
    fold::{CloneFold, DebugFold, DisplayFold, ErrorFold},
    type_set::{Contains, DrainInto, EnumRuntime, Narrow, TupleForm, TypeSet},
    Cons, End,
};

/* ------------------------- OneOf ----------------------- */

/// `OneOf` is an open sum type. It differs from an enum
/// in that you do not need to define any actual new type
/// in order to hold some specific combination of variants,
/// but rather you simply describe the OneOf as holding
/// one value out of several specific possibilities,
/// defined by using a tuple of those possible variants
/// as the generic parameter for the `OneOf`.
///
/// For example, a `OneOf<(String, u32)>` contains either
/// a `String` or a `u32`. The value over a simple `Result`
/// or other traditional enum starts to become apparent in larger
/// codebases where error handling needs to occur in
/// different places for different errors. `OneOf` allows
/// you to quickly specify a function's return value as
/// involving a precise subset of errors that the caller
/// can clearly reason about.
pub struct OneOf<E: TypeSet> {
    pub(crate) value: E::Enum,
}

fn _send_sync_error_assert() {
    use std::io;

    fn is_send<T: Send>(_: &T) {}
    fn is_sync<T: Sync>(_: &T) {}
    fn is_error<T: Error>(_: &T) {}

    let o: OneOf<(io::Error,)> = OneOf::new(io::Error::other("yooo"));
    is_send(&o);
    is_sync(&o);
    is_error(&o);
}

unsafe impl<E> Send for OneOf<E>
where
    E: TypeSet,
    E::Enum: Send,
{
}
unsafe impl<E> Sync for OneOf<E>
where
    E: TypeSet,
    E::Enum: Sync,
{
}

impl<T1> Deref for OneOf<(T1,)>
where
    T1: 'static,
{
    type Target = T1;

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
    fn from(t: T1) -> OneOf<(T1,)> {
        OneOf::new(t)
    }
}

impl<E> Clone for OneOf<E>
where
    E: TypeSet + EnumRuntime,
    E::Variants: Clone + CloneFold,
{
    fn clone(&self) -> Self {
        let boxed = E::Variants::clone_fold(E::enum_ref_as_any(&self.value));
        OneOf {
            value: E::enum_from_any(boxed),
        }
    }
}
impl<E> fmt::Debug for OneOf<E>
where
    E: TypeSet + EnumRuntime,
    E::Variants: fmt::Debug + DebugFold,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        E::Variants::debug_fold(E::enum_ref_as_any(&self.value), formatter)
    }
}

impl<E> fmt::Display for OneOf<E>
where
    E: TypeSet + EnumRuntime,
    E::Variants: fmt::Display + DisplayFold,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        E::Variants::display_fold(E::enum_ref_as_any(&self.value), formatter)
    }
}

impl<E> Error for OneOf<E>
where
    E: TypeSet + EnumRuntime,
    E::Variants: Error + DebugFold + DisplayFold + ErrorFold,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        E::Variants::source_fold(E::enum_ref_as_any(&self.value))
    }
}

impl<E> OneOf<E>
where
    E: TypeSet,
{
    /// Wraps a value in a `OneOf`, inferring the variant from the value's type.
    ///
    /// The compiler verifies at compile time (via the [`Contains`] bound) that `T` is
    /// one of the declared variants. A type mismatch is a compile error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use terrors::OneOf;
    ///
    /// let e: OneOf<(String, u32)> = OneOf::new(42u32);
    /// ```
    pub fn new<T, Index>(t: T) -> OneOf<E>
    where
        T: Any,
        E: EnumRuntime,
        E::Variants: Contains<T, Index>,
    {
        OneOf {
            value: E::enum_from_any(Box::new(t)),
        }
    }

    /// Attempt to downcast the `OneOf` into a specific type, and
    /// if that fails, return a `OneOf` which does not contain that
    /// type as one of its possible variants.
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
        let boxed = E::enum_into_any(self.value);

        if boxed.is::<Target>() {
            Ok(*boxed.downcast::<Target>().unwrap())
        } else {
            type RemainderTuple<E, Target, Index> =
                <<E as TypeSet>::Variants as Narrow<Target, Index>>::Remainder;

            Err(OneOf {
                value: <<RemainderTuple<E, Target, Index> as TupleForm>::Tuple as EnumRuntime>::enum_from_any(boxed),
            })
        }
    }

    /// For a `OneOf` with a single variant, return the contained value.
    pub fn take<Target>(self) -> Target
    where
        Target: 'static,
        E: TypeSet<Variants = Cons<Target, End>> + EnumRuntime,
    {
        *E::enum_into_any(self.value).downcast::<Target>().unwrap()
    }

    /// Consumes the [`Self`] and converts whichever variant it holds into `O`,
    /// requiring every possible variant to implement `Into<O>`.
    ///
    /// We keep this as an inherent method instead of implementing `Into<O> for OneOf<E>`
    /// because that impl conflicts with `core`'s blanket `Into` implementation and
    /// is rejected by coherence rules.
    pub fn into<O>(self) -> O
    where
        E: DrainInto<O>,
    {
        <E as DrainInto<O>>::drain(self)
    }

    /// Convert the `OneOf` to an owned enum for
    /// use in pattern matching etc...
    pub fn to_enum(self) -> E::Enum {
        self.value
    }

    /// Borrow the enum as an enum for use in
    /// pattern matching etc...
    pub fn as_enum<'a>(&'a self) -> E::EnumRef<'a>
    where
        E::EnumRef<'a>: From<&'a Self>,
    {
        E::EnumRef::from(self)
    }
}
