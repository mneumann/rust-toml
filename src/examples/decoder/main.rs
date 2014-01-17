extern mod extra;
extern mod toml = "toml#0.1";

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
    println!("{:s}", value.to_str());

    let cfg: Config = toml::from_toml(value);
    println!("{:s}", cfg.to_str());
}
