use std::io::mem::MemReader;
use std::io::File;

mod toml;

fn main() {
  let contents = File::open(&Path::new(std::os::args()[1])).read_to_end();
  let mut rd = MemReader::new(contents);

  let mut builder = toml::ValueBuilder::new();
  let mut parser = toml::Parser::new(&mut rd);
  parser.parse(&mut builder);
  println!("{:s}", builder.get_root().to_str());
}
