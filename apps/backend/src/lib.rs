//! Platform backend library.
//!
//! The library target exists so that integration tests can build the real router
//! with the real middleware stack instead of duplicating it.

pub mod config;
pub mod http;
pub mod startup;
pub mod state;

#[doc(hidden)]
pub mod testing;
