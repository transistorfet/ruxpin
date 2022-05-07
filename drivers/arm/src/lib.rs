#![no_std]

pub mod gic;
pub mod timer;

pub use timer::SystemTimer;
pub use gic::GenericInterruptController;

