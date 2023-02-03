pub mod automata;
#[cfg(feature = "server")]
mod opts;
mod runtime;

#[cfg(feature = "server")]
pub use opts::Opts;
pub use runtime::run;
