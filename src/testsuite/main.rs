// TOML test suite for [1]
//
// [1]: https://github.com/BurntSushi/toml-test
//
// If given a path, it will perform the same tests like toml-test [1],
// so no need to install "go" :).

extern crate serialize;
extern crate collections;
extern crate toml = "github.com/mneumann/rust-toml#toml";

use serialize::json;
use serialize::json::{Json,String,List,Object};

use collections::treemap::TreeMap;
use std::os;
use std::path::Path;
use std::io::fs::walk_dir;
use std::io::File;

fn to_json_type(typ: &str, val: Json) -> Json {
    let mut tree = box TreeMap::new();
    tree.insert("type".to_string(), String(typ.to_string()));
    tree.insert("value".to_string(), val);
    Object(tree)
}

fn format_float(f: f64) -> String {
    let str = format!("{:.15f}", f);
    let str = str.as_slice();
    let str = str.trim_right_chars('0');
    if str.ends_with(".") {
      format!("{}0", str)
    } else {
      str.to_string()
    }
}

fn to_json(v: &toml::Value) -> Json {
    match v {
        &toml::NoValue => { fail!("Invalid toml document"); }
        &toml::Table(ref map) | &toml::TableInner(ref map) => {
            let mut tree = box TreeMap::new();
            for (k, v) in map.iter() {
                tree.insert(k.clone(), to_json(v));
            }
            Object(tree)
        }
        &toml::TableArray(ref arr) => {
            List(arr.iter().map(|i| to_json(i)).collect())
        }
        &toml::Array(ref arr) => {
            let list = arr.iter().map(|i| to_json(i)).collect();
            to_json_type("array", List(list))
        }
        &toml::Boolean(true) => { to_json_type("bool", String("true".to_string())) }
        &toml::Boolean(false) => { to_json_type("bool", String("false".to_string())) }
        &toml::PosInt(n) => { to_json_type("integer", String(n.to_str())) }
        &toml::NegInt(n) => { to_json_type("integer", String(format!("-{:u}", n))) }
        &toml::Float(n) => { to_json_type("float", String(format_float(n))) }
        &toml::String(ref str) => { to_json_type("string", String(str.clone())) }
        &toml::Datetime(y,m,d,h,mi,s) => {
            let s = format!("{:04u}-{:02u}-{:02u}T{:02u}:{:02u}:{:02u}Z", y,m,d,h,mi,s);
            to_json_type("datetime", String(s))
        }
    }
}

fn toml_test_runner() {
    let toml = toml::parse_from_bytes(std::io::stdin().read_to_end().unwrap().as_slice()).unwrap();
    let json = to_json(&toml);
    println!("{:s}", json.to_pretty_str());
}

fn independent_test_runner(path: Path) {
  let mut tests: int = 0;
  let mut failed: int = 0;
  let mut passed: int = 0;

  for filename in walk_dir(&path.join("invalid")).unwrap() {
    if filename.is_file() && filename.extension_str() == Some("toml") {
      println!("TEST/INVALID: {}", filename.filename_display());
      tests += 1;

      match toml::parse_from_path(&filename) {
          Err(_) => {
              passed += 1;
              println!("   [PASS]");
          }
          _ => {
              failed += 1;
              println!("   [FAIL]");
          }
      }
    }
  }

  for filename in walk_dir(&path.join("valid")).unwrap() {
    if filename.is_file() && filename.extension_str() == Some("toml") {
      let jsonfile = filename.with_extension("json");
      if !jsonfile.is_file() { fail!() }

      println!("TEST/VALID:   {}", filename.filename_display());

      let jsonbytes = File::open(&Path::new(jsonfile)).read_to_end().unwrap();
      let jsonstr = std::str::from_utf8(jsonbytes.as_slice()).unwrap();

      let result = json::from_str(jsonstr);
      if result.is_err() { fail!() }

      tests += 1;

      let json = result.unwrap();
      let toml = toml::parse_from_path(&filename);
      let toml_json = toml.map(|t| to_json(&t));

      let has_failed = 
          match toml_json {
              Ok(ref toml_json_inner) => {
                  toml_json_inner != &json
              }
              Err(_) => { true }
          };

      if has_failed {
          println!("===============================================");
          println!("{:s}", json.to_pretty_str());
          println!("-----------------------------------------------");
          match toml_json {
              Ok(json) => println!("{:s}", json.to_pretty_str()),
              Err(toml::ParseError) => println!("(parse error)"),
              Err(toml::ParseErrorInField(field)) => println!("(parse error in `{}`)", field),
              Err(toml::IOError(e)) => println!("({})", e)
          }
          println!("===============================================");
          failed += 1;
          println!("   [FAIL]");
      } else {
          passed += 1;
          println!("   [PASS]");
      }
    }
  }

  println!("");
  println!("Tests/PASS/FAIL: {:d}/{:d}/{:d}", tests, passed, failed);
  if failed > 0 { fail!(); }
}

fn main() {
    match os::args().len() {
      1 => toml_test_runner(),
      2 => independent_test_runner(Path::new(os::args().get(1).clone())),
      _ => fail!("USAGE: {:s} [path]", os::args().get(0).as_slice()),
    }
}
