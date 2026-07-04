//! Code generation: turns a [`crate::ResolvedComponent`] into the compiled JS
//! module described in `docs/output-design.md`.
//!
//! Built as passes: [`skeleton`] computes the static HTML + dynamic positions;
//! the JS emitter (next pass) turns that plus the resolved reactivity into the
//! final module text.

pub mod skeleton;

pub use skeleton::{build_skeleton, DynamicElement, InsertPos, Skeleton, Slot, SlotContent};
