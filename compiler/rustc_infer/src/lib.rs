//! This crates defines the type inference engine.
//!
//! - **Type inference.** The type inference code can be found in the `infer` module;
//!   this code handles low-level equality and subtyping operations. The
//!   type check pass in the compiler is found in the `rustc_hir_analysis` crate.
//!
//! For more information about how rustc works, see the [rustc dev guide].
//!
//! [rustc dev guide]: https://rustc-dev-guide.rust-lang.org/
//!
//! # Note
//!
//! This API is completely unstable and subject to change.

#![doc(html_root_url = "https://doc.rust-lang.org/nightly/nightly-rustc/")]
#![doc(rust_logo)]
#![feature(rustdoc_internals)]
#![allow(internal_features)]
#![allow(rustc::diagnostic_outside_of_impl)]
#![allow(rustc::untranslatable_diagnostic)]
#![feature(box_patterns)]
#![feature(control_flow_enum)]
#![feature(extend_one)]
#![feature(let_chains)]
#![feature(if_let_guard)]
#![feature(iter_intersperse)]
#![feature(iterator_try_collect)]
#![feature(try_blocks)]
#![feature(yeet_expr)]
#![recursion_limit = "512"] // For rustdoc

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate rustc_middle;

mod errors;
pub mod infer;
pub mod traits;

rustc_fluent_macro::fluent_messages! { "../messages.ftl" }
