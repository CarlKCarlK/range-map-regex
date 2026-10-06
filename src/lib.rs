#[cfg(feature = "declare")]
pub mod declare;
pub mod dfa;
pub mod display;
pub mod fsm;
pub mod grammar;
mod state_id_set;

// `const-structures` macros expand through this path; see `declare`.
#[cfg(feature = "declare")]
#[doc(hidden)]
pub use const_structures::expand as __const_structures_expand;
