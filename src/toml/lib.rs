#[crate_id = "toml#0.1"];
#[desc = "A TOML configuration file parser for Rust"];
#[license = "MIT"];
#[crate_type = "lib"];

pub use toml::{parse_from_bytes,parse_from_buffer,parse_from_file,
               Value,NoValue,True,False,Unsigned,Integer,Float,
               String,Array,Datetime,TableArray,Table};

pub mod toml;
