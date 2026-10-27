// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Original planning probe for spec 009 (research P3): literal inference through
// `impl Into<Level>`. Stand-alone; compile with `rustc --edition 2024 --cfg <case>`.
#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level(u8);

impl Level {
    pub const fn new(v: u8) -> Self {
        assert!(v <= 30, "level must be 0..=30");
        Level(v)
    }
}

#[cfg(any(case_a, case_b))]
impl From<u8> for Level {
    fn from(v: u8) -> Self {
        Level::new(v)
    }
}

// case_b: a second infallible source type, e.g. added later for convenience.
#[cfg(case_b)]
impl From<u16> for Level {
    fn from(v: u16) -> Self {
        Level::new(u8::try_from(v).unwrap())
    }
}

fn parent_at_level(level: impl Into<Level>) -> Level {
    level.into()
}

fn parent_at_level_plain(level: Level) -> Level {
    level
}

#[cfg(case_const)]
const BAD: Level = Level::new(31);

fn main() {
    // A literal through `impl Into<Level>`.
    #[cfg(any(case_a, case_b, case_c))]
    println!("{:?}", parent_at_level(5));
    // A plain parameter with a const-evaluated constructor.
    println!("{:?}", parent_at_level_plain(const { Level::new(5) }));
    #[cfg(case_const)]
    println!("{:?}", BAD);
}
