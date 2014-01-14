extern mod extra;
extern mod toml = "toml#0.1";

use extra::serialize::Decodable;

#[deriving(ToStr,Decodable)]
struct Config {
    host: ~str,
    port: Option<uint>
}

fn main() {
    let toml = r###"
        host = "localhost"
        port = 8080
    "###;

    let value = toml::parse_from_bytes(toml.as_bytes().to_owned());
    let cfg: Config = Decodable::decode(&mut toml::Decoder::new(&value));

    println!("{:s}", value.to_str());
    println!("{:s}", cfg.to_str());
}
