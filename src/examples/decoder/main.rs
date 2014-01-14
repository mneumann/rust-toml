extern mod extra;
extern mod toml = "toml#0.1";

use extra::serialize::Decodable;

#[deriving(ToStr,Decodable)]
struct Config {
    host: ~str,
    port: Option<uint>,
    ids: ~[uint],
    products: ~[Product]
}

#[deriving(ToStr,Decodable)]
struct Product {
    id: uint,
    name: ~str
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

    let value = toml::parse_from_bytes(toml.as_bytes().to_owned());
    let cfg: Config = Decodable::decode(&mut toml::Decoder::new(&value));

    println!("{:s}", value.to_str());
    println!("{:s}", cfg.to_str());
}
