/// A TOML [1] configuration file parser
///
/// Copyright (c) 2014 by Michael Neumann
///
/// [1]: https://github.com/mojombo/toml

use std::io::Buffer;
use std::hashmap::HashMap;
use std::char;
use std::io::mem::MemReader;

#[deriving(ToStr,Clone)]
pub enum Value {
    True,
    False,
    Unsigned(u64),
    Integer(i64),
    Float(f64),
    String(~str),
    Array(~[Value]),
    Datetime, // XXX
    Table(~[Value]),
    Map(HashMap<~str, Value>)
}

pub trait Visitor {
    fn section(&mut self, name: ~str, is_array: bool) -> bool;
    fn pair(&mut self, key: ~str, val: Value) -> bool;
}

pub struct ValueBuilder {
    root: HashMap<~str, Value>,
    current_section: ~str,
    section_is_array: bool
}

impl ValueBuilder {
    pub fn new() -> ValueBuilder {
        ValueBuilder { root: HashMap::new(), current_section: ~"", section_is_array: false }
    }

    fn insert(&mut self, path: &str, value: Value) -> bool {
        let path: ~[&str] = path.split_str(".").collect();
        assert!(path.len() > 0);

        return ValueBuilder::ins(path, &mut self.root, value);
    }

    fn ins(path: &[&str], ht: &mut HashMap<~str, Value>, val: Value) -> bool {
        assert!(path.len() > 0);

        let head = path.head().to_owned();

        if path.len() == 1 {
            let ok = ht.insert(head, val);
            if !ok {
                debug!("Duplicate key");
            }
            return ok;
        }
        else {
            let m = ht.find_or_insert(head, Map(HashMap::new())); // Optimize
            match *m {
                Map(ref mut map) => {
                    return ValueBuilder::ins(path.slice_from(1), map, val);
                }
                _ => {
                    debug!("Wrong type/duplicate key");
                    return false;
                }
            }
        }
    }

    pub fn get_root<'a>(&'a self) -> &'a HashMap<~str, Value> {
        return &self.root;
    }
}

impl Visitor for ValueBuilder {
    fn section(&mut self, name: ~str, is_array: bool) -> bool {
        let ok = self.insert(name, Map(HashMap::new()));
        if !ok {
            debug!("Duplicate key: {}", name);
        }

        self.current_section = name;
        self.section_is_array = is_array; // XXX: not implemented yet

        return ok;
    }

    fn pair(&mut self, key: ~str, val: Value) -> bool {
        if self.current_section.len() == 0 {
            let ok = self.insert(key, val);
            if !ok {
                debug!("Duplicate key: {}", key);
            }
            return ok;
        } else {
            let path = self.current_section + "." + key;
            let ok = self.insert(path, val);
            if !ok {
                debug!("Duplicate key: {}", path);
            }
            return ok;
        }
    }
}

pub struct Parser<'a, BUF> {
    rd: &'a mut BUF,
    current_char: Option<char>
}

impl<'a, BUF: Buffer> Parser<'a, BUF> {
    pub fn new(rd: &'a mut BUF) -> Parser<'a, BUF> {
        let ch = rd.read_char();
        Parser { rd: rd, current_char: ch }
    }

    pub fn parse_from_buffer(rd: &mut BUF) -> Option<Value> {
        let mut builder = ValueBuilder::new();
        let mut parser = Parser::new(rd);
        if parser.parse(&mut builder) {
            return Some(Map(builder.get_root().clone()));
        } else {
            return None;
        }
    }

    pub fn parse_from_bytes(bytes: ~[u8]) -> Option<Value> {
        let mut rd = MemReader::new(bytes);
        return Parser::parse_from_buffer(&mut rd);
    }

    fn advance(&mut self) {
        self.current_char = self.rd.read_char();
    }

    fn ch(&self) -> Option<char> {
        return self.current_char;
    }

    fn eos(&self) -> bool {
        return self.current_char.is_none();
    }

    fn advance_if(&mut self, c: char) -> bool {
        match self.ch() {
            Some(ch) if ch == c => {
               self.advance();
               true
            }
            _ => {
                false
            }
        } 
    }

    fn read_digit(&mut self, radix: uint) -> Option<u8> {
        if self.eos() { return None }
        match char::to_digit(self.ch().unwrap(), radix) {
            Some(n) => {
                self.advance();
                Some(n as u8)
            }
            None => { None }
        }
    }

    fn read_digits(&mut self) -> Option<u64> {
        let mut num: u64;
        match self.read_digit(10) {
            Some(n) => { num = n as u64; }
            None => { return None }
        }
        loop {
            match self.read_digit(10) {
                Some(n) => {
                    // XXX: check range
                    num = num * 10 + (n as u64);
                }
                None => {
                    return Some(num)
                }
            }
        }
    }

    // allows a single "."
    fn read_float_mantissa(&mut self) -> f64 {
        let mut num: f64 = 0.0;
        let mut div: f64 = 10.0;

        loop {
            match self.read_digit(10) {
                Some(n) => {
                    num = num + (n as f64)/div;
                    div = div * 10.0;
                }
                None => {
                    return num;
                }
            }
        }
    }

    fn parse_value(&mut self) -> Option<Value> {
        self.skip_whitespaces();

        if self.eos() { return None }
        match self.ch().unwrap() {
            '-' => {
                self.advance();
                match self.read_digits() {
                    Some(n) => {
                        if self.ch() == Some('.') {
                            // floating point
                            self.advance();
                            let num = self.read_float_mantissa();
                            let num = (n as f64) + num;
                            return Some(Float(-num));
                        }
                        else {
                            match n.to_i64() {
                                Some(i) => Some(Integer(-i)),
                                None => None // XXX: Use Result
                            }
                        }
                    }
                    None => {
                        return None
                    }
                }
            }
            '0' .. '9' => {
                match self.read_digits() {
                    Some(n) => {
                        match self.ch() {
                            Some('.') => {
                                // floating point
                                self.advance();
                                let num = self.read_float_mantissa();
                                let num = (n as f64) + num;
                                return Some(Float(num));
                            }
                            Some('-') => {
                                // XXX
                                fail!("Datetime not yet supported");
                            }
                            _ => {
                                return Some(Unsigned(n))
                            }
                        }
                    }
                    None => {
                        assert!(false);
                        return None
                    }
                }
            }
            't' => {
                self.advance();
                if self.advance_if('r') &&
                   self.advance_if('u') &&
                   self.advance_if('e') {
                    return Some(True)
                } else {
                    return None
                }
            }
            'f' => {
                self.advance();
                if self.advance_if('a') &&
                   self.advance_if('l') &&
                   self.advance_if('s') && 
                   self.advance_if('e') {
                    return Some(True)
                } else {
                    return None
                }
            }
            '[' => {
                self.advance();
                let mut arr = ~[];
                loop {
                    match self.parse_value() {
                        Some(val) => {
                            arr.push(val);
                        }
                        None => {
                            break;
                        }
                    }
                    
                    self.skip_whitespaces_and_comments();
                    if !self.advance_if(',') { break }
                }
                self.skip_whitespaces_and_comments();
                if self.advance_if(']') {
                    return Some(Array(arr));
                } else {
                    return None;
                }
            }
            '"' => {
                match self.parse_string() {
                    Some(str) => { return Some(String(str)) }
                    None => { return None }
                }
            }
            _ => { return None }
        }
    }

    fn parse_string(&mut self) -> Option<~str> {
        if !self.advance_if('"') { return None }

        let mut str = ~"";
        loop {
            if self.ch().is_none() { return None }
            match self.ch().unwrap() {
                '\r' | '\n' | '\u000C' | '\u0008' => { return None }
                '\\' => {
                    self.advance();
                    if self.ch().is_none() { return None }
                    match self.ch().unwrap() {
                        'b' => { str.push_char('\u0008'); self.advance() },
                        't' => { str.push_char('\t'); self.advance() },
                        'n' => { str.push_char('\n'); self.advance() },
                        'f' => { str.push_char('\u000C'); self.advance() },
                        'r' => { str.push_char('\r'); self.advance() },
                        '"' => { str.push_char('"'); self.advance() },
                        '/' => { str.push_char('/'); self.advance() },
                        '\\' => { str.push_char('\\'); self.advance() },
                        'u' => {
                            self.advance();
                            let d1 = self.read_digit(16);
                            let d2 = self.read_digit(16);
                            let d3 = self.read_digit(16);
                            let d4 = self.read_digit(16);
                            match (d1, d2, d3, d4) {
                                (Some(d1), Some(d2), Some(d3), Some(d4)) => {
                                    // XXX: how to construct an UTF character
                                    let ch = (((((d1 as u32 << 8) | d2 as u32) << 8) | d3 as u32) << 8) | d4 as u32;
                                    match char::from_u32(ch) {
                                        Some(ch) => {
                                            str.push_char(ch);
                                        }
                                        None => {
                                            return None;
                                        }
                                    }
                                }
                                _ => return None
                            }
                        }
                        _ => { return None }
                    }
                }
                '"' => {
                    self.advance();
                    return Some(str);
                }
                c => {
                    str.push_char(c);
                    self.advance();
                }
            }
        }
    }

    fn read_token(&mut self, f: |char| -> bool) -> ~str {
        let mut token = ~"";
        loop {
            match self.ch() {
                Some(ch) => {
                    if f(ch) { token.push_char(ch) }
                    else { break }
                }
                None => { break }
            }
            self.advance();
        }

        return token;
    }

    fn parse_section_identifier(&mut self) -> ~str {
        self.read_token(|ch| {
            match ch {
                'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '.' | '_' => true,
                _ => false
            }
        })
    }

    fn skip_whitespaces(&mut self) {
        loop {
            match self.ch() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                    self.advance();
                }
                _ => { break }
            }
        }
    }

    fn skip_whitespaces_and_comments(&mut self) {
        loop {
            match self.ch() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                    self.advance();
                }
                Some('#') => {
                    self.skip_comment();
                }
                _ => { break }
            }
        }
    }

    fn skip_comment(&mut self) {
        assert!(self.ch() == Some('#'));
        // skip to end of line
        loop {
            self.advance();
            match self.ch() {
                Some('\n') => { break }
                None => { return }
                _ => { /* skip */ }
            }
        }
        self.advance();
    }

    pub fn parse<V: Visitor>(&mut self, visitor: &mut V) -> bool {
        loop {
            if self.eos() { return true }
            match self.ch().unwrap() {
                // ignore whitespace
                '\r' | '\n' | ' ' | '\t' => {
                    self.advance();
                }

                // comment
                '#' => {
                    self.skip_comment();
                }

                // section
                '[' => {
                    self.advance();
                    let mut double_section = false;
                    match self.ch() {
                        Some('[') => {
                            double_section = true;
                            self.advance();
                        }
                        _ => {}
                    }

                    let section_name = self.parse_section_identifier();

                    if !self.advance_if(']') { return false }
                    if double_section {
                        if !self.advance_if(']') { return false }
                    }

                    visitor.section(section_name, double_section);
                }

                // identifier
                'a' .. 'z' | 'A' .. 'Z' | '_' => {

                    let ident = self.read_token(|ch| {
                        match ch {
                            'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '_' => true,
                            _ => false
                        }
                    });

                    self.skip_whitespaces();

                    if !self.advance_if('=') { return false } // assign wanted
                    
                    match self.parse_value() {
                        Some(val) => { visitor.pair(ident, val); }
                        None => { return false; }
                    }
                    // do not advance!
                }

                _ => { return false }
            } /* end match */
        }

        assert!(false);
    }
}
