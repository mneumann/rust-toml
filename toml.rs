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
    Datetime(u16,u8,u8,u8,u8,u8),
    TableArray(~[Value]),
    Table(HashMap<~str, Value>)
}

impl Value {
    pub fn get_bool(&self) -> Option<bool> {
        match self {
            &True => { Some(true) }
            &False => { Some(false) }
            _ => { None }
        }
    }

    pub fn get_integer(&self) -> Option<i64> {
        match self {
            &Unsigned(u) => { Some(u as i64) } // XXX
            &Integer(i) => { Some(i) }
            _ => { None }
        }
    }


    pub fn get_float(&self) -> Option<f64> {
        match self {
            &Float(num) => { Some(num) }
            _ => { None } 
        }
    }

    pub fn get_str<'a>(&'a self) -> Option<&'a ~str> {
        match self {
            &String(ref str) => { Some(str) }
            _ => { None } 
        }
    }

    pub fn lookup_key<'a>(&'a self, key: &str) -> Option<&'a Value> {
        match self {
            &Table(ref map) => {
                map.find_equiv(&key)
            }
            _ => { None }
        }
    }

    pub fn lookup_vec<'a>(&'a self, idx: uint) -> Option<&'a Value> {
        match self {
            &Array(ref ary) => {
                ary.get_opt(idx)
            }
            _ => { None }
        }
    }

    pub fn lookup_idx<'a>(&'a self, idx: uint) -> Option<&'a Value> {
        match self {
            &TableArray(ref tableary) => {
                tableary.get_opt(idx)
            }
            _ => { None }
        }
    }

    pub fn lookup_path<'a>(&'a self, path: &[&str]) -> Option<&'a Value> {
        if path.is_empty() {
            Some(self)
        } else {
            self.lookup_key(path[0]).and_then(|a| a.lookup_path(path.slice_from(1)))
        }
    }
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
            if ht.contains_key(&head) {
                match val {
                    TableArray(table_array) => {
                        assert!(table_array.len() == 1);
                        // Special case [[key]], which merges with existing TableArray.
                        match ht.find_mut(&head) {
                            Some(&TableArray(ref mut table_array2)) => {
                                assert!(table_array2.len() > 0);
                                table_array2.push(table_array[0]);
                                return true;
                            }
                            _ => { }
                        }
                    }
                    _ => { }
                }

                debug!("Duplicate key");
                return false;
            }
            else {
                let ok = ht.insert(head, val);
                assert!(ok);
                return true;
            }
        }
        else {
            match ht.find_mut(&head) {
                Some(&Table(ref mut table)) => {
                    return ValueBuilder::ins(path.slice_from(1), table, val);
                }
                Some(&TableArray(ref mut table_array)) => {
                    assert!(table_array.len() > 0);
                    let mut last_table = &mut table_array[table_array.len()-1];
                    match last_table {
                        &Table(ref mut hmap) => {
                            return ValueBuilder::ins(path.slice_from(1), hmap, val);
                        }
                        _ => {
                            // TableArray's only contain Table's
                            assert!(false);
                        }
                    }
                }
                Some(_) => {
                    debug!("Wrong type/duplicate key");
                    return false;
                }
                None => {
                    // fallthrough
                }
            }
            let mut table = HashMap::new();
            let ok = ValueBuilder::ins(path.slice_from(1), &mut table, val);
            ht.insert(head, Table(table));
            return ok;
        }
    }

    pub fn get_root<'a>(&'a self) -> &'a HashMap<~str, Value> {
        return &self.root;
    }
}

impl Visitor for ValueBuilder {
    fn section(&mut self, name: ~str, is_array: bool) -> bool {
        let ok = if is_array {
            self.insert(name, TableArray(~[Table(HashMap::new())]))
        } else {
            self.insert(name, Table(HashMap::new()))
        };
        if !ok {
            debug!("Duplicate key: {}", name);
        }

        self.current_section = name;

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
            return Some(Table(builder.get_root().clone()));
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

    fn read_two_digits(&mut self) -> Option<u8> {
        let d1 = self.read_digit(10);
        let d2 = self.read_digit(10);
        match (d1, d2) {
            (Some(d1), Some(d2)) => Some(d1*10+d2),
            _ => None
        }
    }

    fn read_digits(&mut self) -> (Option<u64>, uint) {
        let mut num: u64;
        match self.read_digit(10) {
            Some(n) => { num = n as u64; }
            None => { return (None, 0) }
        }
        let mut ndigits = 1;
        loop {
            match self.read_digit(10) {
                Some(n) => {
                    // XXX: check range
                    num = num * 10 + (n as u64);
                    ndigits += 1;
                }
                None => {
                    return (Some(num), ndigits)
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
                    (Some(n), _) => {
                        if self.ch() == Some('.') {
                            // floating point
                            self.advance();
                            let num = self.read_float_mantissa();
                            let num = (n as f64) + num;
                            return Some(Float(-num));
                        }
                        else {
                            match n.to_i64() {
                                Some(i) => return Some(Integer(-i)),
                                None => return None // XXX: Use Result
                            }
                        }
                    }
                    (None, _) => {
                        return None
                    }
                }
            }
            '0' .. '9' => {
                match self.read_digits() {
                    (Some(n), ndigits) => {
                        match self.ch() {
                            Some('.') => {
                                // floating point
                                self.advance();
                                let num = self.read_float_mantissa();
                                let num = (n as f64) + num;
                                return Some(Float(num));
                            }
                            Some('-') => {
                                if ndigits != 4 {
                                    debug!("Invalid Datetime");
                                    return None;
                                }
                                self.advance();

                                let year = n;

                                let month = self.read_two_digits();
                                if month.is_none() || !self.advance_if('-') {
                                    debug!("Invalid Datetime");
                                    return None;
                                }

                                let day = self.read_two_digits();
                                if day.is_none() || !self.advance_if('T'){
                                    debug!("Invalid Datetime");
                                    return None;
                                }

                                let hour = self.read_two_digits();
                                if hour.is_none() || !self.advance_if(':') {
                                    debug!("Invalid Datetime");
                                    return None;
                                }

                                let min = self.read_two_digits();
                                if min.is_none() || !self.advance_if(':') {
                                    debug!("Invalid Datetime");
                                    return None;
                                }

                                let sec = self.read_two_digits();
                                if sec.is_none() || !self.advance_if('Z') {
                                    debug!("Invalid Datetime");
                                    return None;
                                }

                                match (year, month, day, hour, min, sec) {
                                    (y, Some(m), Some(d),
                                     Some(h), Some(min), Some(s))
                                    if m > 0 && m <= 12 && d > 0 && d <= 31 &&
                                       h <= 24 && min <= 60 && s <= 60 => {
                                        return Some(Datetime(y as u16,m,d,h,min,s))
                                    }
                                    _ => {
                                        debug!("Invalid Datetime range");
                                        return None;
                                    }
                                }
                            }
                            _ => {
                                return Some(Unsigned(n))
                            }
                        }
                    }
                    (None, _) => {
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
                ' ' | '\t' | '\n' | '\r' | '[' | ']' => false,
                _ => true
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

                // identifier: anything else starts an idenfifier!
                // NOTE that we do not allow '.' in identifiers!
                _ => {
                    let ident = self.read_token(|ch| {
                        match ch {
                            ' ' | '\t' | '=' | '.' => false,
                            _ => true
                        }
                    });

                    self.skip_whitespaces();

                    if !self.advance_if('=') { return false } // assign wanted
                    
                    match self.parse_value() {
                        Some(val) => { visitor.pair(ident, val); }
                        None => { return false; }
                    }
                }
            } /* end match */
        }

        assert!(false);
    }
}
