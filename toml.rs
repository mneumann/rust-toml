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

fn read_char_opt(rd: &mut MemReader) -> Option<char> {
  match rd.read_byte() {
    Some(b) => Some(b as char),
    None => None
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
    fn section(&mut self, name: ~str, is_array: bool) -> bool;
    fn pair(&mut self, key: ~str, val: Value) -> bool;
}

struct TOMLVisitor {
    root: HashMap<~str, Value>,
    current_path: ~str,
    is_array: bool
}

impl TOMLVisitor {
    fn new() -> TOMLVisitor {
        TOMLVisitor { root: HashMap::new(), current_path: ~"", is_array: false }
    }
    fn get_root<'a>(&'a self) -> &'a HashMap<~str, Value> {
        return &self.root;
    }
}

impl Visitor for TOMLVisitor {
    fn section(&mut self, name: ~str, is_array: bool) -> bool {
        debug!("Section: {} (is_array={})", name, is_array);
        self.is_array = is_array;
        self.current_path = name;
        return true
    }
    fn pair(&mut self, key: ~str, val: Value) -> bool {
        debug!("Pair: {} {:s}", key, val.to_str());
        let mut m = self.root.find_or_insert(self.current_path.clone(), Map(HashMap::new())); // XXX: remove clone
        match *m {
            Map(ref mut map) => {
                let ok = map.insert(key, val);
                return ok
            }
            _ => { return false }
        }
    }
}

fn read_token(rd: &mut MemReader, current_char: Option<char>, f: |char| -> bool) -> (~str, Option<char>) {
    let mut current_char = current_char;
    let mut token = ~"";
    loop {
        match current_char {
            Some(ch) => {
                if f(ch) { token.push_char(ch) }
                else { break }
            }
            None => { break }
        }
        current_char = read_char_opt(rd); // advance
    }

    return (token, current_char);
}

fn parse_section_identifier(rd: &mut MemReader, current_char: Option<char>) -> (~str, Option<char>) {
    read_token(rd, current_char, |ch| {
        match ch {
            'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '.' | '_' => true,
            _ => false
        }
    })
}

fn skip_whitespaces(rd: &mut MemReader, current_char: Option<char>) -> Option<char> {
    let mut current_char = current_char;
    loop {
        match current_char {
            Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                current_char = read_char_opt(rd); // advance
            }
            _ => { break }
        }
    }
    return current_char;
}

fn parse<V: Visitor>(rd: &mut MemReader, visitor: &mut V) -> bool {
    let mut current_char: Option<char> = read_char_opt(rd);

    loop {
        if current_char.is_none() { return true }
        let ch = current_char.unwrap();
        match ch {
            // ignore whitespace
            '\r' | '\n' | ' ' | '\t' => { }

            // comment
            '#' => {
                // skip to end of line
                loop {
                    current_char = read_char_opt(rd);
                    match current_char {
                        Some('\n') => { break }
                        None => { return true }
                        _ => { /* skip */ }
                    }
                }
            }

            // section
            '[' => {
                current_char = read_char_opt(rd); // advance
                let mut double_section = false;
                match current_char {
                    Some('[') => {
                        double_section = true;
                        current_char = read_char_opt(rd); // advance
                    }
                    _ => {}
                }

                let (section_name, ch) = parse_section_identifier(rd, current_char);
                current_char = ch;

                match current_char {
                    Some(']') => { /* ok */ }
                    _ => { return false }
                }

                if double_section {
                    current_char = read_char_opt(rd); // advance
                    if current_char != Some(']') { return false }
                }

                visitor.section(section_name, double_section);
            }

            // identifier
            'a' .. 'z' | 'A' .. 'Z' | '_' => {

                let (ident, ch) = read_token(rd, current_char, |ch| {
                    match ch {
                        'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '_' => true,
                        _ => false
                    }
                });

                current_char = ch;
                current_char = skip_whitespaces(rd, current_char);

                // assign wanted
                if current_char != Some('=') { return false }
                
                // XXX: current_char = read_char_opt(rd); // advance
                visitor.pair(ident, parse_value(rd)); // XXX: parse_value is not using peeking yet
            }

            _ => { return false }
        } /* end match */
        current_char = read_char_opt(rd); // advance
    }

    assert!(false);
}

fn main() {
  let contents = File::open(&Path::new(std::os::args()[1])).read_to_end();
  let mut rd = MemReader::new(contents);
  let mut visitor = TOMLVisitor::new();
  parse(&mut rd, &mut visitor);
  println!("{:s}", visitor.get_root().to_str());
}
