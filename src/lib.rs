pub mod cli;
pub mod core;

mod context;
mod files;
mod needle;
mod output;
mod params;

#[cfg(not(feature = "bench"))]
mod matching;
#[cfg(not(feature = "bench"))]
mod net;
#[cfg(not(feature = "bench"))]
mod netlike;
#[cfg(not(feature = "bench"))]
mod scanner;

#[cfg(feature = "bench")]
pub mod matching;
#[cfg(feature = "bench")]
pub mod net;
#[cfg(feature = "bench")]
pub mod netlike;
#[cfg(feature = "bench")]
pub mod scanner;
