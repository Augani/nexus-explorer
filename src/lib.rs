#![recursion_limit = "2048"]

#[cfg(not(test))]
pub mod app;
pub mod io;
pub mod models;
pub mod utils;
#[cfg(not(test))]
pub mod views;
