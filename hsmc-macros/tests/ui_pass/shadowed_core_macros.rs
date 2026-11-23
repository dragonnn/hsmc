//! Caller macros must not replace the generated core assertions and helpers.
#![allow(unused_macros)]

macro_rules! assert {
    ($($tokens:tt)*) => {
        ::core::compile_error!("caller assert macro expanded")
    };
}
macro_rules! debug_assert {
    ($($tokens:tt)*) => {
        ::core::compile_error!("caller debug_assert macro expanded")
    };
}
macro_rules! panic {
    ($($tokens:tt)*) => {
        ::core::compile_error!("caller panic macro expanded")
    };
}
macro_rules! matches {
    ($($tokens:tt)*) => {
        ::core::compile_error!("caller matches macro expanded")
    };
}
macro_rules! unreachable {
    ($($tokens:tt)*) => {
        ::core::compile_error!("caller unreachable macro expanded")
    };
}

mod chart {
    use hsmc::{statechart, Duration};

    pub struct Data {
        entries: u8,
    }

    #[derive(Clone)]
    pub enum Event {
        Go,
        Halt,
    }

    statechart! {
        Probe {
            context: Data;
            events: Event;
            terminate(Halt);
            default(Idle);
            state Idle {
                entry: entered;
                on(Go) => Active;
            }
            state Active {
                entry: entered;
                on(after Duration::from_millis(1)) => Idle;
            }
        }
    }

    impl ProbeIdleActions for ProbeActionContext<'_> {
        fn entered(&mut self) {
            self.entries += 1;
        }
    }

    impl ProbeActiveActions for ProbeActionContext<'_> {
        fn entered(&mut self) {
            self.entries += 1;
        }
    }

    pub fn run() {
        let mut machine = Probe::new(Data { entries: 0 });
        let _ = machine.step(Duration::ZERO);
        machine.send(Event::Go).unwrap();
        let _ = machine.step(Duration::ZERO);
        let _ = machine.step(Duration::from_millis(1));
        ::core::assert_eq!(machine.context().entries, 3);
        machine.send(Event::Halt).unwrap();
        let _ = machine.step(Duration::ZERO);
        ::core::assert!(machine.is_terminated());
    }
}

fn main() {
    chart::run();
}
