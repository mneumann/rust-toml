use std::io::mem::MemReader;
use std::io::buffered::BufferedReader;
use std::io::File;
use toml::Parser;

mod toml;

fn main() {
  let path = Path::new(std::os::args()[1]);
  let mut file = File::open(&path);

  //let contents = file.read_to_end();
  let mut rd = BufferedReader::new(file);
  let value = Parser::parse_from_buffer(&mut rd);

  println!("{:s}", value.to_str());
}
