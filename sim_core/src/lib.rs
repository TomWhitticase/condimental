//! Milestone 1 scaffold: proves the Rust (gdext) <-> Godot round trip.
//!
//! This is deliberately not real simulation code (see ARCHITECTURE.md §9,
//! milestone 1) - `SimBridge` here is a stub exposing one trivial method so
//! GDScript can call into Rust and get a known value back.

use godot::prelude::*;

struct SauceFactoryExtension;

#[gdextension]
unsafe impl ExtensionLibrary for SauceFactoryExtension {}

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
struct SimBridge {
    base: Base<RefCounted>,
}

#[godot_api]
impl SimBridge {
    /// Trivial round-trip proof: call from GDScript, get a known value back.
    #[func]
    fn ping(&self) -> i64 {
        42
    }
}
