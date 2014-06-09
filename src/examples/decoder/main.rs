extern crate serialize;
extern crate toml = "github.com/mneumann/rust-toml#toml";

use std::os;

#[deriving(Show,Decodable)]
struct Config {
    host: String,
    port: Option<uint>,
    ids: Vec<uint>,
    products: Vec<Product>
}

#[deriving(Show,Decodable)]
struct Product {
    id: uint,
    name: String
}

fn main() {
    let toml = r###"
        host = "localhost"
        ids = [1, 10, 20] 
          [[products]]
          id = 1
          name = "Product 1" 
          [[products]]
          id = 2
          name = "Product 2"
    "###;

    let value = match toml::parse_from_bytes(toml.as_bytes()) {
        Ok(v) => v,
        Err(toml::ParseError) => {
            println!("parse error");
            os::set_exit_status(1);
            return;
        }
        Err(toml::IOError(e)) => {
            println!("I/O error: {}", e);
            os::set_exit_status(1);
            return;
        }
    };
    println!("{}", value);

    let cfg: Config = toml::from_toml(value).unwrap();
    println!("{:s}", cfg.to_str());
}
