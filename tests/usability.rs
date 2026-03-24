//! Tests for usability of the public API.
//! These are not meant to be comprehensive, but should cover common patterns and ensure that the ergonomics are good.
#![allow(
    clippy::dbg_macro,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::unwrap_used,
    clippy::unwrap_in_result,
    clippy::tests_outside_test_module,
    clippy::let_underscore_must_use
)]

use terrors::{Broaden as _, OneOf, SubsetErr as _};

#[derive(Debug)]
struct NotEnoughMemory;

#[derive(Debug)]
struct Timeout;

#[derive(Debug)]
struct RetriesExhausted;

#[test]
fn retry() {
    fn inner() -> Result<(), OneOf<(NotEnoughMemory, RetriesExhausted)>> {
        for _ in 0..3 {
            let Err(err) = does_stuff() else {
                return Ok(());
            };

            match err.narrow::<Timeout, _>() {
                Ok(_timeout) => {}
                Err(allocation_oneof) => {
                    println!("didn't get Timeout, now trying to get NotEnoughMemory");
                    let allocation_oneof: OneOf<(NotEnoughMemory,)> = allocation_oneof;
                    let allocation = allocation_oneof.narrow::<NotEnoughMemory, _>().unwrap();

                    return Err(OneOf::new(allocation));
                }
            }
        }

        Err(OneOf::new(RetriesExhausted))
    }

    let _ = dbg!(inner());
}

#[expect(clippy::let_unit_value)]
fn does_stuff() -> Result<(), OneOf<(NotEnoughMemory, Timeout)>> {
    // TODO Try impl after superset type work
    let _allocation = match allocates() {
        Ok(a) => a,
        Err(e) => return Err(e.broaden()),
    };

    // TODO Try impl after superset type work
    let _chat = match chats() {
        Ok(c) => c,
        Err(e) => return Err(OneOf::new(e)),
    };

    Ok(())
}

fn allocates() -> Result<(), OneOf<(NotEnoughMemory,)>> {
    let result: Result<(), NotEnoughMemory> = Err(NotEnoughMemory);

    result?;

    Ok(())
}

const fn chats() -> Result<(), Timeout> {
    Err(Timeout)
}

#[test]
fn smoke() {
    let o_1: OneOf<(u32, &'static str)> = OneOf::new(5_u32);
    let _narrowed_1: u32 = o_1.narrow::<u32, _>().unwrap();

    let o_2: OneOf<(&'static str, u32)> = OneOf::new(5_u32);
    let _narrowed_2: u32 = o_2.narrow::<u32, _>().unwrap();

    let o_3: OneOf<(&'static str, u32)> = OneOf::new("5");
    let _narrowed_3: OneOf<(&'static str,)> = o_3.narrow::<u32, _>().unwrap_err();

    let o_4: OneOf<(&'static str, u32)> = OneOf::new("5");

    let _: &'static str = o_4.narrow().unwrap();

    let o_5: OneOf<(&'static str, u32)> = OneOf::new("5");
    o_5.narrow::<&'static str, _>().unwrap();

    let o_6: OneOf<(&'static str, u32)> = OneOf::new("5");
    let o_7: OneOf<(u32, &'static str)> = o_6.broaden();
    let o_8: OneOf<(&'static str, u32)> = o_7.subset().unwrap();
    let _: OneOf<(u32, &'static str)> = o_8.subset().unwrap();

    let o_9: OneOf<(u8, u16, u32)> = OneOf::new(3_u32);
    let _: Result<OneOf<(u16,)>, OneOf<(u8, u32)>> = o_9.subset();
    let o_10: OneOf<(u8, u16, u32)> = OneOf::new(3_u32);
    let _: Result<u16, OneOf<(u8, u32)>> = o_10.narrow();
}

#[test]
fn debug() {
    use core::error::Error as _;
    use std::io;

    let o_1: OneOf<(u32, &'static str)> = OneOf::new(5_u32);

    // Debug is implemented if all types in the type set implement Debug
    dbg!(&o_1);

    // Display is implemented if all types in the type set implement Display
    println!("{o_1}");

    type E = io::Error;
    let e = io::Error::other("wuaaaaahhhzzaaaaaaaa");

    let o_2: OneOf<(E,)> = OneOf::new(e);

    // std::error::Error is implemented if all types in the type set implement it
    dbg!(o_2.source());

    let o_3: OneOf<(u32, &'static str)> = OneOf::new("hey");
    dbg!(o_3);
}

#[test]
fn multi_match() {
    use terrors::E2;

    let o_1: OneOf<(u32, &'static str)> = OneOf::new(5_u32);

    match o_1.as_enum() {
        E2::T1(u) => {
            println!("handling {u}: u32");
        }
        E2::T2(s) => {
            println!("handling {s}:  &'static str");
        }
    }

    match o_1.to_enum() {
        E2::T1(u) => {
            println!("handling {u}: u32");
        }
        E2::T2(s) => {
            println!("handling {s}:  &'static str");
        }
    }
}

#[test]
fn multi_narrow() {
    use terrors::E2;

    struct Timeout;
    struct Backoff;

    let o_1: OneOf<(u8, u16, u32, u64, u128)> = OneOf::new(5_u32);

    let _narrow_res: Result<OneOf<(u8, u128)>, OneOf<(u16, u32, u64)>> = o_1.subset();

    let o_2: OneOf<(u8, u16, Backoff, Timeout, u32, u64, u128)> = OneOf::new(Timeout {});

    match o_2.subset::<(Timeout, Backoff), _>().unwrap().to_enum() {
        E2::T1(Timeout {}) => {
            println!(":)");
        }
        E2::T2(Backoff {}) => {
            unreachable!()
        }
    }
}

#[test]
fn into() {
    let o: OneOf<(u32,)> = OneOf::new(7_u32);
    let drained: u32 = o.into();

    assert_eq!(drained, 7);

    let from_str: OneOf<(&'static str,)> = OneOf::new("hello");
    let from_owned: OneOf<(&'static str,)> = OneOf::new("world");

    let drained_1: &'static str = from_str.into();
    let drained_2: &'static str = from_owned.into();

    assert_eq!(drained_1, "hello");
    assert_eq!(drained_2, "world");

    let from_u8: OneOf<(u8, u16, u32)> = OneOf::new(3_u8);
    let from_u32: OneOf<(u8, u16, u32)> = OneOf::new(42_u32);

    let drained_1: u128 = from_u8.into();
    let drained_2: u128 = from_u32.into();

    assert_eq!(drained_1, 3_u128);
    assert_eq!(drained_2, 42_u128);
}

#[test]
fn complex_into() {
    struct NotEnoughMemory;
    struct Timeout;
    struct CommonHandleableError;
    struct NeverUsed;

    enum BroadError {
        NotEnoughMemory(NotEnoughMemory),
        Timeout(Timeout),
        CommonHandlableError(CommonHandleableError),
        _NeverUsed(NeverUsed),
    }
    impl From<NotEnoughMemory> for BroadError {
        fn from(value: NotEnoughMemory) -> Self {
            Self::NotEnoughMemory(value)
        }
    }
    impl From<Timeout> for BroadError {
        fn from(value: Timeout) -> Self {
            Self::Timeout(value)
        }
    }
    impl From<CommonHandleableError> for BroadError {
        fn from(value: CommonHandleableError) -> Self {
            Self::CommonHandlableError(value)
        }
    }

    // even From impl for NeverUsed isn't necessary since it's never used

    fn does_stuff() -> Result<(), OneOf<(Timeout,)>> {
        Err(OneOf::new(Timeout))
    }
    fn do_handleable_stuff() -> Result<(), OneOf<(NotEnoughMemory, CommonHandleableError)>> {
        Err(OneOf::new(CommonHandleableError))
    }
    fn do_another_stuff() -> Result<(), OneOf<(NotEnoughMemory,)>> {
        Err(OneOf::new(NotEnoughMemory))
    }
    fn mr_delegation() -> Result<(), OneOf<(NotEnoughMemory, Timeout)>> {
        do_another_stuff().broaden()?;

        let result = do_handleable_stuff();

        // compile error since CommonHandleableError wasn't in return type
        // result.map_err(OneOf::broaden)?;

        match result.unwrap_err().narrow() {
            Ok(CommonHandleableError) => {
                // handling common handleable error
            }
            Err(oneof) => {
                // CommonHandleableError wasn't here, so it's totally fine
                return Err(oneof.broaden());
            }
        }

        does_stuff().broaden()
    }

    let o = mr_delegation().unwrap_err();
    let _the_broad_one: BroadError = o.into();
}

#[test]
fn broad_err_basic() {
    let e: Result<(), u8> = Err(9_u8);
    let b: Result<(), OneOf<(u8, u16)>> = e.map_err(OneOf::new);

    let extracted: u8 = b.unwrap_err().narrow().unwrap();
    assert_eq!(extracted, 9);
}

#[test]
fn broad_err_ok_passthrough() {
    let e: Result<u32, u8> = Ok(42);
    let b: Result<u32, OneOf<(u8, u16)>> = e.map_err(OneOf::new);

    assert_eq!(b.unwrap(), 42);
}

#[test]
fn broad_err_non_first_variant() {
    let e: Result<(), u16> = Err(7_u16);
    let b: Result<(), OneOf<(u8, u16, u32)>> = e.map_err(OneOf::new);

    let extracted: u16 = b.unwrap_err().narrow().unwrap();
    assert_eq!(extracted, 7);
}

#[test]
fn broad_err_question_mark() {
    fn returns_plain() -> Result<(), u8> {
        Err(5_u8)
    }
    fn returns_plain_u16() -> Result<(), u16> {
        Err(3_u16)
    }

    fn combined() -> Result<(), OneOf<(u8, u16)>> {
        returns_plain().map_err(OneOf::new)?;
        returns_plain_u16().map_err(OneOf::new)?;
        Ok(())
    }

    let err = combined().unwrap_err();
    let extracted: u8 = err.narrow().unwrap();
    assert_eq!(extracted, 5);
}

#[test]
fn ext_broaden_option() {
    let e: Option<OneOf<(u8,)>> = Some(OneOf::new(7_u8));
    let b: Option<OneOf<(u8, u16)>> = e.broaden();

    assert!(b.is_some());
    let extracted: u8 = b.unwrap().narrow().unwrap();
    assert_eq!(extracted, 7);
}

#[test]
fn ext_broaden_result_err() {
    let e: Result<(), OneOf<(u8,)>> = Err(OneOf::new(9_u8));
    let b: Result<(), OneOf<(u8, u16)>> = e.broaden();

    assert!(b.is_err());
    let extracted: u8 = b.unwrap_err().narrow().unwrap();
    assert_eq!(extracted, 9);
}

#[test]
fn ext_subset_result_split() {
    type SplitResult = Result<Result<u32, OneOf<(u16,)>>, OneOf<(u8,)>>;

    let r1: Result<u32, OneOf<(u8, u16)>> = Ok(10);
    let split_1: SplitResult = r1.subset();
    assert_eq!(split_1.unwrap().unwrap(), 10);

    let r2: Result<u32, OneOf<(u8, u16)>> = Err(OneOf::new(7_u8));
    let split_2: SplitResult = r2.subset();
    assert_eq!(split_2.unwrap_err().narrow().unwrap(), 7_u8);

    let r3: Result<u32, OneOf<(u8, u16)>> = Err(OneOf::new(9_u16));
    let split_3: SplitResult = r3.subset();
    let rest = split_3.unwrap().unwrap_err();
    assert_eq!(rest.narrow().unwrap(), 9_u16);
}

#[test]
fn complex_subset() {
    #[derive(Debug)]
    struct NotEnoughMemory;
    #[derive(Debug)]
    struct Timeout;
    #[derive(Debug)]
    struct CommonHandleableError;

    #[derive(Clone, Copy)]
    enum Mode {
        Common,
        Memory,
        Timeout,
    }

    fn always_times_out() -> Result<(), OneOf<(Timeout,)>> {
        Err(OneOf::new(Timeout))
    }

    fn do_handleable_stuff(
        mode: Mode,
    ) -> Result<(), OneOf<(NotEnoughMemory, CommonHandleableError, Timeout)>> {
        match mode {
            Mode::Common => Err(OneOf::new(CommonHandleableError)),
            Mode::Memory => Err(OneOf::new(NotEnoughMemory)),
            Mode::Timeout => Err(OneOf::new(Timeout)),
        }
    }

    fn mr_delegation(mode: Mode) -> Result<(), OneOf<(Timeout, NotEnoughMemory)>> {
        let rest_or_ok = {
            let this = do_handleable_stuff(mode);
            match this {
                Ok(value) => Ok(Ok(value)),
                Err(errs) => match errs.subset() {
                    Ok(o) => Err(o),
                    Err(rest) => Ok(Err(rest)),
                },
            }
        }?;

        match rest_or_ok {
            Ok(()) => {}
            Err(rest) => match rest.to_enum() {
                terrors::E1::T1(common) => {
                    println!("handling common error: {common:?}");
                }
            },
        }

        always_times_out().broaden()
    }

    let timeout_after_local_handling = mr_delegation(Mode::Common).unwrap_err();
    timeout_after_local_handling.narrow::<Timeout, _>().unwrap();

    let propagated_memory = mr_delegation(Mode::Memory).unwrap_err();
    propagated_memory.narrow::<NotEnoughMemory, _>().unwrap();

    let propagated_timeout = mr_delegation(Mode::Timeout).unwrap_err();
    propagated_timeout.narrow::<Timeout, _>().unwrap();
}

#[cfg(feature = "nightly")]
#[test]
fn nightly_new_keeps_nested_layout_compatible() {
    let inner: OneOf<(u8, u16)> = OneOf::new(5_u8);
    let nested: OneOf<((), OneOf<(u8, u16)>)> = OneOf::new(inner);
    let flat: OneOf<((), u8, u16)> = nested.broaden();

    let extracted: u8 = flat.narrow().unwrap();
    assert_eq!(extracted, 5);
}

#[cfg(feature = "nightly")]
#[test]
fn nightly_broaden_flattens_nested_layout() {
    let inner: OneOf<(u8, u16)> = OneOf::new(9_u8);
    let nested: OneOf<((), OneOf<(u8, u16)>)> = OneOf::new(inner);
    let flat: OneOf<((), u8, u16)> = nested.broaden();

    let extracted: u8 = flat.narrow().unwrap();
    assert_eq!(extracted, 9);
}
