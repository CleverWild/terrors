//! Fold traits for heterogeneous dispatch over `Cons`/`End` type lists.
//!
//! Each fold trait mirrors one `std` trait (`Error`, `Display`, `Debug`, `Clone`)
//! using a method that dispatches at runtime on `dyn Any` — allowing `OneOf`
//! to forward those traits without knowing the concrete type it holds.
use core::any::Any;
use core::fmt;
use std::error::Error;

use crate::{Cons, End};

/* ------------------------- Error ----------------------- */

impl<Head, Tail> Error for Cons<Head, Tail>
where
    Head: Error,
    Tail: Error,
{
}

/// Runtime dispatch of [`Error::source`] (and optionally [`Error::provide`]) over a `Cons` list.
///
/// Walks the `Cons<Head, Tail>` chain at runtime by downcasting `any` to `Head`;
/// if the downcast succeeds it delegates to `Head`'s [`Error`] impl, otherwise
/// it recurses into `Tail`. The base case on [`End`] is unreachable because `End`
/// is uninhabited.
pub trait ErrorFold {
    fn source_fold(any: &dyn Any) -> Option<&(dyn Error + 'static)>;

    #[cfg(feature = "error_provide")]
    fn provide_fold<'a>(any: &'a dyn Any, request: &mut std::error::Request<'a>);
}

impl ErrorFold for End {
    fn source_fold(_: &dyn Any) -> Option<&(dyn Error + 'static)> {
        unreachable!("source_fold called on End");
    }

    #[cfg(feature = "error_provide")]
    fn provide_fold<'a>(_: &dyn Any, _: &mut std::error::Request<'a>) {
        unreachable!("provide_fold called on End");
    }
}

impl<Head, Tail> ErrorFold for Cons<Head, Tail>
where
    Cons<Head, Tail>: Error,
    Head: 'static + Error,
    Tail: ErrorFold,
{
    fn source_fold(any: &dyn Any) -> Option<&(dyn Error + 'static)> {
        if let Some(head_ref) = any.downcast_ref::<Head>() {
            head_ref.source()
        } else {
            Tail::source_fold(any)
        }
    }

    #[cfg(feature = "error_provide")]
    fn provide_fold<'a>(any: &'a dyn Any, request: &mut std::error::Request<'a>) {
        if let Some(head_ref) = any.downcast_ref::<Head>() {
            head_ref.provide(request)
        } else {
            Tail::provide_fold(any, request)
        }
    }
}

/* ------------------------- Display ----------------------- */

impl<Head, Tail> fmt::Display for Cons<Head, Tail>
where
    Head: fmt::Display,
    Tail: fmt::Display,
{
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!("Display called for Cons which is not constructable")
    }
}

/// Runtime dispatch of [`fmt::Display`] over a `Cons` list.
///
/// Downcasts `any` to each `Head` type in turn until one matches, then
/// calls `fmt::Display::fmt` on that value. Used by the [`fmt::Display`]
/// implementation of [`OneOf`].
pub trait DisplayFold {
    fn display_fold(any: &dyn Any, formatter: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl DisplayFold for End {
    fn display_fold(_: &dyn Any, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!("display_fold called on End");
    }
}

impl<Head, Tail> DisplayFold for Cons<Head, Tail>
where
    Cons<Head, Tail>: fmt::Display,
    Head: 'static + fmt::Display,
    Tail: DisplayFold,
{
    fn display_fold(any: &dyn Any, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(head_ref) = any.downcast_ref::<Head>() {
            head_ref.fmt(formatter)
        } else {
            Tail::display_fold(any, formatter)
        }
    }
}

/* ------------------------- Debug ----------------------- */

/// Runtime dispatch of [`fmt::Debug`] over a `Cons` list.
///
/// Mirrors [`DisplayFold`] but delegates to `fmt::Debug::fmt` instead.
/// Used by the [`fmt::Debug`] implementation of [`OneOf`].
pub trait DebugFold {
    fn debug_fold(any: &dyn Any, formatter: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl DebugFold for End {
    fn debug_fold(_: &dyn Any, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!("debug_fold called on End");
    }
}

impl<Head, Tail> DebugFold for Cons<Head, Tail>
where
    Cons<Head, Tail>: fmt::Debug,
    Head: 'static + fmt::Debug,
    Tail: DebugFold,
{
    fn debug_fold(any: &dyn Any, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(head_ref) = any.downcast_ref::<Head>() {
            head_ref.fmt(formatter)
        } else {
            Tail::debug_fold(any, formatter)
        }
    }
}

/* ------------------------- Clone ----------------------- */

impl<Head, Tail> Clone for Cons<Head, Tail>
where
    Head: 'static + Clone,
    Tail: CloneFold,
{
    fn clone(&self) -> Self {
        unreachable!("clone called for Cons which is not constructable");
    }
}

/// Runtime dispatch of [`Clone`] over a `Cons` list.
///
/// Downcasts `any` to the first matching `Head` type, clones it, and returns
/// the clone as a new `Box<dyn Any>`. Used by the [`Clone`] implementation of
/// [`OneOf`].
pub trait CloneFold {
    fn clone_fold(any: &dyn Any) -> Box<dyn Any>;
}

impl CloneFold for End {
    fn clone_fold(_: &dyn Any) -> Box<dyn Any> {
        unreachable!("clone_fold called on End");
    }
}

impl<Head, Tail> CloneFold for Cons<Head, Tail>
where
    Head: 'static + Clone,
    Tail: CloneFold,
{
    fn clone_fold(any: &dyn Any) -> Box<dyn Any> {
        if let Some(head_ref) = any.downcast_ref::<Head>() {
            Box::new(head_ref.clone())
        } else {
            Tail::clone_fold(any)
        }
    }
}

/* ------------------------- IsFold ----------------------- */

/// Runtime membership check over a `Cons` list.
///
/// Returns `true` if `any` is an instance of any type in the list.
/// Used by [`OneOf::subset`] to decide whether the held value belongs
/// to a target subset before attempting the conversion.
pub trait IsFold {
    fn is_fold(any: &dyn Any) -> bool;
}

impl IsFold for End {
    fn is_fold(_: &dyn Any) -> bool {
        false
    }
}

impl<Head, Tail> IsFold for Cons<Head, Tail>
where
    Head: 'static,
    Tail: IsFold,
{
    fn is_fold(any: &dyn Any) -> bool {
        if any.is::<Head>() {
            true
        } else {
            Tail::is_fold(any)
        }
    }
}
