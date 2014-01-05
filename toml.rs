/// A TOML [1] configuration file parser
///
/// [1]: https://github.com/mojombo/toml

use std::io::mem::MemReader;
use std::io::File;
use std::hashmap::HashMap;

fn read_char(rd: &mut MemReader) -> char {
  match rd.read_byte() {
    Some(b) => b as char,
    None => fail!()
  }
}

#[deriving(ToStr)]
enum Value {
    True,
    False,
    Unsigned(u64),
    Integer(i64),
    Float(f64),
    String(~str),
    Array(~[Value]),
    Datetime, // XXX
    Map(HashMap<~str, Value>) // XXX: This is no value
}

// parse values recursivly
fn parse_value(rd: &mut MemReader) -> Value {
    enum State {
        st_value,
        st_number,
        st_got_number
    }

    let mut state: State = st_value;
    let mut val: ~str = ~"";

    loop {
        match state {
            st_value => {
                if rd.eof() { fail!() }
                let ch = read_char(rd);
                match ch { 
                    '\r' | '\n' | ' ' | '\t' => { } 
                    '0' .. '9' => {
                        val.push_char(ch);
                        state = st_number;
                    }
                    't' => {
                        assert!(read_char(rd) == 'r');
                        assert!(read_char(rd) == 'u');
                        assert!(read_char(rd) == 'e');
                        return True
                    }
                    'f' => {
                        assert!(read_char(rd) == 'a');
                        assert!(read_char(rd) == 'l');
                        assert!(read_char(rd) == 's');
                        assert!(read_char(rd) == 'e');
                        return False
                    }
                    '"' => {
                        loop {
                            let ch = read_char(rd);
                            if ch == '"' {
                                break;
                            }
                            val.push_char(ch);
                        }
                        return String(val)
                    }
                    _ => { fail!() }
                }
            }
            st_got_number => {
                return Unsigned(from_str(val).unwrap())
            }
            st_number => {
                if rd.eof() {
                    state = st_got_number;
                } else {
                    let ch = read_char(rd);
                    match ch { 
                        '\r' | '\n' | ' ' | '\t' => { state = st_got_number }
                        '#' => { skip_comment(rd); state = st_got_number } 
                        '0' .. '9' => {
                            val.push_char(ch);
                        }
                        _ => { fail!() }
                    }
                }
            }
        }
    }
}


// We must be already within the '#"
fn skip_comment(rd: &mut MemReader) {
    loop {
        if rd.eof() { return }
        match read_char(rd) {
            '\n' => { return }
            _ => { }
        }
    }
}

trait Visitor {
    fn section(&mut self, name: &str, is_array: bool);
    fn pair(&mut self, key: &str, val: Value); 
}

struct MyVisitor {
    root: HashMap<~str, Value>,
    current_path: ~str
}

impl MyVisitor {
    fn new() -> MyVisitor {
        MyVisitor { root: HashMap::new(), current_path: ~"" }
    }
    fn get_root<'a>(&'a self) -> &'a HashMap<~str, Value> {
        return &self.root;
    }
}

impl Visitor for MyVisitor {
    fn section(&mut self, name: &str, is_array: bool) {
        assert!(is_array == false);
        debug!("Section: {}", name);
        self.current_path = name.to_owned(); //clone();
    }
    fn pair(&mut self, key: &str, val: Value) {
        debug!("Pair: {} {:s}", key, val.to_str());
        let mut m = self.root.find_or_insert(self.current_path.clone(), Map(HashMap::new())); // XXX: remove clone
        match *m {
            Map(ref mut map) => {
                let fresh = map.insert(key.to_owned(), val);
                assert!(fresh == true);
            }
            _ => { fail!("Invalid TOML") }
        }
    }
}

fn parse<V: Visitor>(rd: &mut MemReader, visitor: &mut V) {
    enum State {
        st_toplevel,
        st_comment,
        st_section,
        st_ident,
        st_assign_wanted,
        st_value_wanted
    }

    let mut state = st_toplevel;
    let mut buf: ~str = ~"";

    loop {
        match state {
            st_toplevel => {
                if rd.eof() { break }
                let ch = read_char(rd);
                match ch {
                    // ignore whitespace
                    '\r' | '\n' | ' ' | '\t' => { }

                    // comment
                    '#' => { state = st_comment }

                    // section
                    '[' => { state = st_section }

                    // identifier
                    'a' .. 'z' | 'A' .. 'Z' | '_' => {
                        state = st_ident;
                        buf.push_char(ch)
                    }

                    _ => { fail!() }
                }
            }
            st_comment => {
                if rd.eof() { break }
                match read_char(rd) {
                    '\n' => { state = st_toplevel }
                    _ => { }
                }
            }
            st_section => {
                if rd.eof() { fail!() }
                let ch = read_char(rd);
                match ch {
                    'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '.' | '_'=> {
                        buf.push_char(ch);
                    }
                    ']' => {
                        visitor.section(buf, false);
                        buf.truncate(0);
                        state = st_toplevel;
                    }
                    _ => { fail!() }
                }
            }
            st_ident => {
                if rd.eof() { fail!() }
                let ch = read_char(rd);
                match ch {
                    'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '_'=> {
                        buf.push_char(ch);
                    }

                    '\r' | '\n' | ' ' | '\t' => { state = st_assign_wanted }

                    '=' => { state = st_value_wanted }

                    _ => { fail!() }
                }
            }
            st_assign_wanted => {
                if rd.eof() { fail!() }
                match read_char(rd) {
                    '\r' | '\n' | ' ' | '\t' => { } 
                    '=' => { state = st_value_wanted }
                    _ => { fail!() }
                }
            }
            st_value_wanted => {
                visitor.pair(buf, parse_value(rd));
                buf.truncate(0);
                state = st_toplevel;
            }
        }
    }
}

fn main() {
  let contents = File::open(&Path::new(std::os::args()[1])).read_to_end();
  let mut rd = MemReader::new(contents);
  let mut visitor = MyVisitor::new();
  parse(&mut rd, &mut visitor);
  println!("{:s}", visitor.get_root().to_str());
}
