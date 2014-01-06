/// A TOML [1] configuration file parser
///
/// Copyright (c) 2014 by Michael Neumann
/// Copyright (c) 2014 by Flavio Percoco
///
/// [1]: https://github.com/mojombo/toml

#[crate_id = "toml#0.1"]
#[comment = "Toml library for Rust"]
#[license = "MIT"]
#[crate_type = "lib"]

extern mod extra;

pub use toml::{parse_from_file,
               parse_from_buffer,
               parse_from_bytes,
               Value};

pub mod toml;
