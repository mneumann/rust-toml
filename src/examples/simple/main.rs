extern crate toml = "github.com/mneumann/rust-toml#toml:0.1";

use std::os;

fn main() {
  if os::args().len() < 2 {
    println!("usage: ./simple input-file");
    os::set_exit_status(1);
    return;
  }
  let value = match toml::parse_from_file(os::args().get(1).as_slice()) {
    Ok(v) => v,
    Err(toml::ParseError) => {
      println!("parse error");
      os::set_exit_status(1);
      return;
    },
    Err(toml::ParseErrorInField(field)) => {
      println!("parse error in field `{}`", field);
      os::set_exit_status(1);
      return;
    },
    Err(toml::IOError(e)) => {
      println!("I/O error: {}", e);
      os::set_exit_status(1);
      return;
    },
  };
  println!("{}", value);

  let a = value.lookup_elm(&("a")).and_then(|a| a.get_str());
  if a.is_some() { println!("Found a: {:?}", a) }

  let abc_def_a = value.lookup_elm(&("abc")).and_then(|a| a.lookup_elm(&("def")).and_then(|a| a.lookup_elm(&("a"))));
  if abc_def_a.is_some() { println!("Found abc.def.a: {:?}", abc_def_a) }

  let a = value.lookup("abc.def.a");
  if a.is_some() { println!("Found a: {:?}", a) }

  match value.lookup("products.0") {
    Some(product_0) => println!("Found product[0]: {:?}", product_0),
    _ => println!("product[0] not found")
  }
}
