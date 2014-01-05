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


trait Visitor {
    fn section(&mut self, name: ~str, is_array: bool) -> bool;
    fn pair(&mut self, key: ~str, val: Value) -> bool;
}

struct TOMLVisitor {
    root: HashMap<~str, Value>,
    current_section: ~str,
    section_is_array: bool
}

impl TOMLVisitor {
    fn new() -> TOMLVisitor {
        TOMLVisitor { root: HashMap::new(), current_section: ~"", section_is_array: false }
    }
    fn get_root<'a>(&'a self) -> &'a HashMap<~str, Value> {
        return &self.root;
    }
}

impl Visitor for TOMLVisitor {
    fn section(&mut self, name: ~str, is_array: bool) -> bool {
        debug!("Section: {} (is_array={})", name, is_array);
        self.section_is_array = is_array;
        self.current_section = name;
        return true
    }
    fn pair(&mut self, key: ~str, val: Value) -> bool {
        debug!("Pair: {} {:s}", key, val.to_str());
        let mut m = self.root.find_or_insert(self.current_section.clone(), Map(HashMap::new())); // XXX: remove clone
        match *m {
            Map(ref mut map) => {
                let ok = map.insert(key, val);
                return ok
            }
            _ => { return false }
        }
    }
}

// parse values recursivly
fn parse_value(rd: &mut MemReader, current_char: Option<char>) -> (Option<Value>, Option<char>) {
    let mut current_char = skip_whitespaces(rd, current_char);

    if current_char.is_none() { return (None, current_char) }
    let ch = current_char.unwrap();
    match ch {
        '0' .. '9' => {
            let (num, ch) = read_token(rd, current_char, |ch| {
                match ch {
                    '0' .. '9' => true,
                    _ => false
                }
            });
            match from_str(num) {
              Some(n) => return (Some(Unsigned(n)), ch),
              None => return (None, ch)
            }
        }
        't' => {
            current_char = read_char_opt(rd);
            if current_char != Some('r') { return (None, current_char) }
            current_char = read_char_opt(rd);
            if current_char != Some('u') { return (None, current_char) }
            current_char = read_char_opt(rd);
            if current_char != Some('e') { return (None, current_char) }
            current_char = read_char_opt(rd);

            return (Some(True), current_char)
        }
        'f' => {
            current_char = read_char_opt(rd);
            if current_char != Some('a') { return (None, current_char) }
            current_char = read_char_opt(rd);
            if current_char != Some('l') { return (None, current_char) }
            current_char = read_char_opt(rd);
            if current_char != Some('s') { return (None, current_char) }
            current_char = read_char_opt(rd);
            if current_char != Some('e') { return (None, current_char) }
            current_char = read_char_opt(rd);

            return (Some(False), current_char)
        }
        '"' => {
            current_char = read_char_opt(rd);
            let (str, ch) = read_token(rd, current_char, |ch| {
                match ch {
                    '"' => false,
                    _ => true
                }
            });
            current_char = ch;

            if current_char != Some('"') { return (None, current_char) } 
            current_char = read_char_opt(rd);
            return (Some(String(str)), current_char)
        }
        _ => { return (None, current_char) }
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
                
                current_char = read_char_opt(rd); // advance
                let (val, ch) = parse_value(rd, current_char);
                current_char = ch;
                match val {
                  Some(v) => { visitor.pair(ident, v); }
                  None => { return false; }
                }
                continue; // do not advance!
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
