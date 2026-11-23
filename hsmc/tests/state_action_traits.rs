//! State traits select independent action implementations and signatures.

#![cfg(not(feature = "embassy"))]

use hsmc::{statechart, Duration};

#[cfg(feature = "tokio")]
macro_rules! methods {
    ($(fn $name:ident($($args:tt)*) $body:block)*) => {
        $(async fn $name($($args)*) $body)*
    };
}

#[cfg(not(feature = "tokio"))]
macro_rules! methods {
    ($($method:tt)*) => { $($method)* };
}

#[cfg(feature = "tokio")]
fn complete<T>(future: impl core::future::Future<Output = T>) -> T {
    use core::task::{Context, Poll, Waker};
    match core::pin::pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("these action bodies should complete immediately"),
    }
}

#[cfg(feature = "tokio")]
macro_rules! drive {
    ($call:expr) => {
        complete($call)
    };
}

#[cfg(not(feature = "tokio"))]
macro_rules! drive {
    ($call:expr) => {
        $call
    };
}

mod ownership {
    use super::*;

    #[derive(Default)]
    pub struct Data {
        log: Vec<&'static str>,
    }

    #[derive(Clone)]
    pub enum Event {
        Go,
        ParentEvent,
        RootEvent,
        Halt,
    }

    statechart! {
        Probe {
            context: Data;
            events: Event;
            terminate(Halt);
            default(Parent);
            entry: shared;
            exit: shared;
            on(RootEvent) => shared;
            on(every Duration::from_millis(3)) => shared;
            state Parent {
                default(Idle);
                entry: shared;
                exit: shared;
                on(ParentEvent) => shared;
                on(every Duration::from_millis(2)) => shared;
                state Idle {
                    entry: shared;
                    exit: shared;
                    on(Go) => Listening;
                }
                state Listening {
                    entry: shared;
                    exit: shared;
                    on(every Duration::from_millis(1)) => shared;
                }
            }
        }
    }

    impl ProbeActions for ProbeActionContext<'_> {
        methods! { fn shared(&mut self) { self.log.push("root"); } }
    }

    impl ProbeParentActions for ProbeActionContext<'_> {
        methods! { fn shared(&mut self) { self.log.push("parent"); } }
    }

    impl ProbeIdleActions for ProbeActionContext<'_> {
        methods! { fn shared(&mut self) { self.log.push("idle"); } }
    }

    impl ProbeListeningActions for ProbeActionContext<'_> {
        methods! { fn shared(&mut self) { self.log.push("listening"); } }
    }

    #[test]
    fn entry_exit_bubbled_handlers_and_timers_use_the_owning_state() {
        let mut machine = Probe::new(Data::default());
        let _ = drive!(machine.step(Duration::ZERO));
        machine.send(Event::ParentEvent).unwrap();
        let _ = drive!(machine.step(Duration::ZERO));
        machine.send(Event::RootEvent).unwrap();
        let _ = drive!(machine.step(Duration::ZERO));
        machine.send(Event::Go).unwrap();
        let _ = drive!(machine.step(Duration::ZERO));
        let _ = drive!(machine.step(Duration::from_millis(3)));
        // Each step handles one expired timer, deepest state first.
        let _ = drive!(machine.step(Duration::ZERO));
        let _ = drive!(machine.step(Duration::ZERO));
        machine.send(Event::Halt).unwrap();
        let _ = drive!(machine.step(Duration::ZERO));

        assert!(machine.is_terminated());
        assert_eq!(
            machine.context().log,
            [
                "root",
                "parent",
                "idle",
                "parent",
                "root",
                "idle",
                "listening",
                "listening",
                "parent",
                "root",
                "listening",
                "parent",
                "root",
            ]
        );
    }
}

mod signatures {
    use super::*;

    #[derive(Default)]
    pub struct Data {
        values: Vec<u64>,
    }

    #[derive(Clone)]
    pub enum Event {
        RootValue(u64),
        Unit,
        Tuple(u16),
        Struct { value: u32 },
        OtherStruct { value: u32 },
        ToB,
        ToC,
        ToD,
        Reset,
    }

    statechart! {
        Probe {
            context: Data;
            events: Event;
            default(A);
            on(RootValue(value: u64)) => capture;
            state A {
                entry: capture;
                on(Unit) => capture;
                on(ToB) => B;
            }
            state B {
                on(Tuple(value: u16)) => capture;
                on(ToC) => C;
            }
            state C {
                on(Struct { value: u32 }) => capture;
                on(OtherStruct { value: u32 }) => capture;
                on(ToD) => D;
            }
            state D {
                on(Reset) => A;
            }
        }
    }

    impl ProbeActions for ProbeActionContext<'_> {
        methods! { fn capture(&mut self, value: u64) { self.values.push(value + 1000); } }
    }

    impl ProbeAActions for ProbeActionContext<'_> {
        methods! { fn capture(&mut self) { self.values.push(1); } }
    }

    impl ProbeBActions for ProbeActionContext<'_> {
        methods! { fn capture(&mut self, value: u16) { self.values.push(u64::from(value) + 10); } }
    }

    impl ProbeCActions for ProbeActionContext<'_> {
        methods! { fn capture(&mut self, value: u32) { self.values.push(u64::from(value) + 100); } }
    }

    // The empty trait exists; the machine does not require its implementation.
    #[allow(dead_code)]
    fn empty_trait_exists<T: ProbeDActions>() {}

    #[test]
    fn same_name_can_have_independent_payload_signatures_in_each_state() {
        let mut machine = Probe::new(Data::default());
        let _ = drive!(machine.step(Duration::ZERO));
        for event in [
            Event::RootValue(5),
            Event::Unit,
            Event::ToB,
            Event::Tuple(2),
            Event::ToC,
            Event::Struct { value: 3 },
            Event::OtherStruct { value: 4 },
            Event::ToD,
            Event::Reset,
        ] {
            machine.send(event).unwrap();
            let _ = drive!(machine.step(Duration::ZERO));
        }
        assert_eq!(machine.context().values, [1, 1005, 1, 12, 103, 104, 1]);
    }
}
