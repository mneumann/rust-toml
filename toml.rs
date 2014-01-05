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
    Map(HashMap<~str, Value>)
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

fn parse(rd: &mut MemReader) -> HashMap<~str, Value> {
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
    let mut path: ~str = ~"";

    let mut root = HashMap::new();

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
                        debug!("Section: {}", buf);
                        path = buf.clone();
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
                debug!("Key: {}", buf);
                let val = parse_value(rd);

                debug!("Value: {:?}", val);
                debug!("Path: {}", path);

                // XXX: split path
                let mut m = root.find_or_insert(path.clone(), Map(HashMap::new())); // XXX: remove clone
                match *m {
                    Map(ref mut map) => {
                        let fresh = map.insert(buf.clone(), val);
                        assert!(fresh == true);
                    }
                    _ => { fail!("Invalid TOML") }
                }

                buf.truncate(0);
                state = st_toplevel;
            }
        }
    }

    return root;
}

fn main() {
  let contents = File::open(&Path::new(std::os::args()[1])).read_to_end();
  let mut rd = MemReader::new(contents);
  let root = parse(&mut rd);
  println!("{:s}", root.to_str());
}
