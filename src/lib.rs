pub mod cli;
pub mod core;

mod context;
mod files;
mod matching;
mod needle;
mod net;
mod output;
mod params;
mod scanner;

#[cfg(not(feature = "bench"))]
mod netlike;

#[cfg(feature = "bench")]
pub mod netlike;
