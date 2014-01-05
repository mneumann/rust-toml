use std::io::mem::MemReader;
use std::io::buffered::BufferedReader;
use std::io::File;

mod toml;

fn main() {
  let path = Path::new(std::os::args()[1]);
  let mut file = File::open(&path);

  //let contents = file.read_to_end();
  //let mut rd = MemReader::new(contents);
  let mut rd = BufferedReader::new(file);

  let mut builder = toml::ValueBuilder::new();
  let mut parser = toml::Parser::new(&mut rd);
  parser.parse(&mut builder);
  println!("{:s}", builder.get_root().to_str());
}
