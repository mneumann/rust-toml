#![crate_id = "github.com/mneumann/rust-toml#toml:0.1"]
#![desc = "A TOML configuration file parser for Rust"]
#![license = "MIT"]
#![crate_type = "lib"]
#![feature(phase)]

/// A TOML [1] configuration file parser
///
/// Copyright (c) 2014 by Michael Neumann (mneumann@ntecs.de)
///
/// [1]: https://github.com/mojombo/toml

extern crate serialize;
extern crate collections;
#[phase(syntax, link)] extern crate log;

use std::char;
use std::mem;

use collections::hashmap::{HashMap,MoveEntries};
use std::slice::MoveItems;

use std::io::{File,IoError,IoResult,EndOfFile};
use std::io::{Buffer,BufReader,BufferedReader};
use std::path::Path;

use serialize::Decodable;

use std::fmt;
use std::strbuf::StrBuf;

#[deriving(Clone)]
pub enum Value {
    NoValue,
    Boolean(bool),
    PosInt(u64),
    NegInt(u64),
    Float(f64),
    String(~str),
    Datetime(u16,u8,u8,u8,u8,u8),
    Array(~[Value]),
    TableArray(~[Value]),
    Table(bool, ~HashMap<~str, Value>) // bool=true iff section already defiend
}

impl fmt::Show for Value {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            NoValue       => write!(fmt.buf, "NoValue"),
            Boolean(b)    => write!(fmt.buf, "Boolean({:b})", b),
            PosInt(n)     => write!(fmt.buf, "PosInt({:u})", n),
            NegInt(n)     => write!(fmt.buf, "NegInt({:u})", n),
            Float(f)      => write!(fmt.buf, "Float({:f})", f),
            String(ref s) => write!(fmt.buf, "String({:s})", s.as_slice()),
            Datetime(a,b,c,d,e,f) =>  {
                write!(fmt.buf, "Datetime({},{},{},{},{},{})", a,b,c,d,e,f)
            }
            Array(ref arr) => write!(fmt.buf, "Array({})", arr.as_slice()),
            TableArray(ref arr) => write!(fmt.buf, "TableArray({})", arr.as_slice()),
            Table(_, ref hm) => write!(fmt.buf, "Table({})", **hm)
        }
    }
}



/// Possible errors returned from the parse functions
#[deriving(Show,Clone,Eq)]
pub enum Error {
    /// An parser error occurred during parsing
    ParseError,
    /// An I/O error occurred during parsing
    IOError(IoError)
}

//
// This function determines if v1 and v2 have compatible ("equivalent") types
// as TOML allows only arrays where all elements are of the same type.
//
fn have_equiv_types(v1: &Value, v2: &Value) -> bool {
    match (v1, v2) {
        (&Boolean(_), &Boolean(_)) => true,
        (&PosInt(_), &PosInt(_)) => true,
        (&PosInt(_), &NegInt(_)) => true,
        (&NegInt(_), &PosInt(_)) => true,
        (&NegInt(_), &NegInt(_)) => true,
        (&Float(_), &Float(_)) => true,
        (&String(_), &String(_)) => true,
        (&Datetime(..), &Datetime(..)) => true,
        (&Array(_), &Array(_)) => true, // Arrays can be heterogenous in TOML
        _ => false
    }
}

enum PathElement<'a> {
    Key(&'a str),
    Idx(uint)
}

trait LookupValue<'a> {
    fn lookup_in(&self, value: &'a Value) -> Option<&'a Value>;
}

impl<'a> LookupValue<'a> for uint {
    fn lookup_in(&self, value: &'a Value) -> Option<&'a Value> {
        match value {
           &TableArray(ref tableary) => {
               tableary.get(*self)
           }
           _ => { None }
        }
    }
}

impl<'a, 'b> LookupValue<'a> for &'b str {
    fn lookup_in(&self, value: &'a Value) -> Option<&'a Value> {
        match value {
            &Table(_, ref map) => {
                map.find_equiv(self)
            }
            _ => { None }
        }
    }
}

impl<'a, 'b> LookupValue<'a> for PathElement<'b> {
    fn lookup_in(&self, value: &'a Value) -> Option<&'a Value> {
        match *self {
            Key(key) => key.lookup_in(value),
            Idx(idx) => idx.lookup_in(value)
        }
    }
}

impl<'a, 'b, 'c> LookupValue<'a> for &'b[PathElement<'c>] {
    fn lookup_in(&self, value: &'a Value) -> Option<&'a Value> {
        match self.head() {
          None => Some(value),
          Some(head) => value.lookup_elm(head).and_then(|a| a.lookup_elm(&self.tail()))
        }
    }
}

impl Value {
    pub fn get_bool(&self) -> Option<bool> {
        match self {
            &Boolean(b) => { Some(b) }
            _ => { None }
        }
    }

    pub fn get_int(&self) -> Option<i64> { // XXX
        match self {
            &PosInt(u) => { Some(u.to_i64().unwrap()) } // XXX
            &NegInt(u) => { Some(-(u.to_i64().unwrap())) } // XXX
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

    pub fn get_vec<'a>(&'a self) -> Option<&'a ~[Value]> {
        match self {
            &Array(ref vec) => { Some(vec) }
            _ => { None }
        }
    }

    pub fn get_table<'a>(&'a self) -> Option<&'a ~HashMap<~str, Value>> {
        match self {
            &Table(_, ref table) => { Some(table) }
            _ => { None }
        }
    }

    pub fn get_table_array<'a>(&'a self) -> Option<&'a ~[Value]> {
        match self {
            &TableArray(ref vec) => { Some(vec) }
            _ => { None }
        }
    }

    pub fn lookup_elm<'a>(&'a self, elm: &LookupValue<'a>) -> Option<&'a Value> {
        elm.lookup_in(self)
    }
 
    pub fn lookup_vec<'a>(&'a self, idx: uint) -> Option<&'a Value> {
        match self {
            &Array(ref ary) => {
                ary.get(idx)
            }
            _ => { None }
        }
    }

    pub fn lookup<'a>(&'a self, path: &'a str) -> Option<&'a Value> {
        let mut curr: Option<&'a Value> = Some(self);

        for p in path.split_str(".") {
          match curr {
            None => break,
            Some(s) => { 
              let elm = match from_str::<uint>(p) {
                Some(idx) => Idx(idx),
                None => Key(p),
              };
              curr = s.lookup_elm(&elm);
            }
          }
        }

        return curr 
    }
}

trait Visitor {
    fn section(&mut self, name: ~str, is_array: bool) -> bool;
    fn pair(&mut self, key: ~str, val: Value) -> bool;
}

struct ValueBuilder<'a> {
    root: &'a mut ~HashMap<~str, Value>,
    current_path: ~[~str]
}

impl<'a> ValueBuilder<'a> {
    fn new(root: &'a mut ~HashMap<~str, Value>) -> ValueBuilder<'a> {
        ValueBuilder { root: root, current_path: ~[] }
    }

    fn recursive_create_tree(path: &[~str], ht: &mut ~HashMap<~str, Value>, is_array: bool) -> bool {
        assert!(path.len() > 0);

        if path.head().unwrap().is_empty() { return false } // don't allow empty keys

        let term_rec: bool = path.len() == 1;

        let head = path.head().unwrap(); // TODO: optimize

        match ht.find_mut(head) {
            Some(&TableArray(ref mut table_array)) => {
                assert!(table_array.len() > 0);

                if term_rec { // terminal recursion
                    if is_array {
                        table_array.push(Table(true, ~HashMap::new()));
                        return true;
                    }
                    else {
                        debug!("Duplicate key");
                        return false;
                    }
                }
                else {
                    //let last_table = &mut ;
                    match table_array[table_array.len()-1] {
                        Table(_, ref mut hmap) => {
                            return ValueBuilder::recursive_create_tree(path.tail(), hmap, is_array);
                        }
                        _ => {
                            // TableArray's only contain Table's
                            unreachable!();
                        }
                    }
                }
            }
            Some(&Table(already_created, ref mut table)) => {
                if term_rec { // terminal recursion
                    if is_array {
                        debug!("Duplicate key");
                        return false;
                    }
                    else {
                        if already_created {
                            debug!("Duplicate section");
                            return false;
                        }
                        return true;
                    }
                }
                else {
                    return ValueBuilder::recursive_create_tree(path.tail(), table, is_array);
                }
            }
            Some(_) => {
                debug!("Wrong type/duplicate key");
                return false;
            }
            None => {
                // fall-through, as we cannot modify 'ht' here
            }
        }

        let value =
        if term_rec { // terminal recursion
            if is_array { TableArray(~[Table(false, ~HashMap::new())]) }
            else { Table(true, ~HashMap::new()) }
        }
        else {
            let mut table = ~HashMap::new();
            let ok = ValueBuilder::recursive_create_tree(path.tail(), &mut table, is_array);
            if !ok { return false }
            Table(false, table)
        };
        let ok = ht.insert(head.to_owned(), value);
        assert!(ok);
        return ok;
    }

    fn insert_value(path: &[~str], key: &str, ht: &mut ~HashMap<~str, Value>, val: Value) -> bool {
        if path.is_empty() {
            return ht.insert(key.to_owned(), val);
        }
        else {
            let head = path.head().unwrap(); // TODO: optimize
            match ht.find_mut(head) {
                Some(&Table(_, ref mut table)) => {
                    return ValueBuilder::insert_value(path.tail(), key, table, val);
                }
                Some(&TableArray(ref mut table_array)) => {
                    assert!(table_array.len() > 0);
                    match table_array[table_array.len()-1] {
                        Table(_, ref mut hmap) => {
                            return ValueBuilder::insert_value(path.tail(), key, hmap, val);
                        }
                        _ => {
                            // TableArray's only contain Table's
                            unreachable!();
                        }
                    }
                }
                _ => {
                    debug!("Wrong type/duplicate key");
                    return false;
                }
            }
        }
    }
}

impl<'a> Visitor for ValueBuilder<'a> {
    fn section(&mut self, name: ~str, is_array: bool) -> bool {
        self.current_path = name.split_str(".").map(|i| i.to_owned()).collect();

        let ok = ValueBuilder::recursive_create_tree(self.current_path.as_slice(), self.root, is_array);
        if !ok {
            debug!("Duplicate section: {}", name);
        }
        return ok;
    }

    fn pair(&mut self, key: ~str, val: Value) -> bool {
        let ok = ValueBuilder::insert_value(self.current_path.as_slice(), key, self.root, val);
        if !ok {
            debug!("Duplicate key: {} in path {:?}", key, self.current_path);
        }
        return ok;
    }
}

struct Parser<'a, BUF> {
    rd: &'a mut BUF,
    current_char: IoResult<char>,
    line: uint
}

impl<'a, BUF: Buffer> Parser<'a, BUF> {
    fn new(rd: &'a mut BUF) -> Parser<'a, BUF> {
        let ch = rd.read_char();
        let mut line = 1;
        if ch == Ok('\n') { line += 1 }
        Parser { rd: rd, current_char: ch, line: line }
    }

    fn advance(&mut self) {
        self.current_char = self.rd.read_char();
    }

    fn get_line(&self) -> uint { self.line }

    fn ch(&self) -> Option<char> {
        match self.current_char {
            Ok(c) => Some(c),
            Err(_) => None
        }
    }

    /// Returns `true` if the input is exhausted (due to EOF or an error)
    fn eos(&self) -> bool {
        return self.current_char.is_err();
    }

    /// Returns any error encountered by the parser. Returns `None` for EndOfFile.
    fn to_err(&self) -> Option<IoError> {
        match self.current_char {
            Ok(_) | Err(IoError{kind: EndOfFile, ..}) => None,
            Err(ref e) => Some(e.clone())
        }
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

    fn parse_float_rest(&mut self, n: u64, mul: f64) -> Value {
        if self.ch().is_none() { return NoValue }
        match self.ch().unwrap() {
            '0' .. '9' => {
                let num = self.read_float_mantissa();
                let num = (n as f64) + num;
                Float(num * mul)
            }
            _ => NoValue
        }
    }

    fn parse_value(&mut self) -> Value {
        self.skip_whitespaces_and_comments();

        if self.eos() { return NoValue }
        match self.ch().unwrap() {
            '-' => {
                self.advance();
                match self.read_digits() {
                    (Some(n), _) => {
                        if self.ch() == Some('.') {
                            // floating point
                            self.advance();
                            return self.parse_float_rest(n, -1.0);
                        }
                        else {
                            return NegInt(n);
                        }
                    }
                    (None, _) => {
                        return NoValue
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
                                return self.parse_float_rest(n, 1.0);
                            }
                            Some('-') => {
                                if ndigits != 4 {
                                    debug!("Invalid Datetime");
                                    return NoValue;
                                }
                                self.advance();

                                let year = n;

                                let month = self.read_two_digits();
                                if month.is_none() || !self.advance_if('-') {
                                    debug!("Invalid Datetime");
                                    return NoValue;
                                }

                                let day = self.read_two_digits();
                                if day.is_none() || !self.advance_if('T'){
                                    debug!("Invalid Datetime");
                                    return NoValue;
                                }

                                let hour = self.read_two_digits();
                                if hour.is_none() || !self.advance_if(':') {
                                    debug!("Invalid Datetime");
                                    return NoValue;
                                }

                                let min = self.read_two_digits();
                                if min.is_none() || !self.advance_if(':') {
                                    debug!("Invalid Datetime");
                                    return NoValue;
                                }

                                let sec = self.read_two_digits();
                                if sec.is_none() || !self.advance_if('Z') {
                                    debug!("Invalid Datetime");
                                    return NoValue;
                                }

                                match (year, month, day, hour, min, sec) {
                                    (y, Some(m), Some(d),
                                     Some(h), Some(min), Some(s))
                                    if m > 0 && m <= 12 && d > 0 && d <= 31 &&
                                       h <= 24 && min <= 60 && s <= 60 => {
                                        return Datetime(y as u16,m,d,h,min,s)
                                    }
                                    _ => {
                                        debug!("Invalid Datetime range");
                                        return NoValue;
                                    }
                                }
                            }
                            _ => {
                                return PosInt(n)
                            }
                        }
                    }
                    (None, _) => {
                        assert!(false);
                        return NoValue
                    }
                }
            }
            't' => {
                self.advance();
                if self.advance_if('r') &&
                   self.advance_if('u') &&
                   self.advance_if('e') {
                    return Boolean(true)
                } else {
                    return NoValue
                }

        }
            'f' => {
                self.advance();
                if self.advance_if('a') &&
                   self.advance_if('l') &&
                   self.advance_if('s') &&
                   self.advance_if('e') {
                    return Boolean(false)
                } else {
                    return NoValue
                }
            }
            '[' => {
                self.advance();
                let mut arr = ~[];
                loop {
                    match self.parse_value() {
                        NoValue => {
                            break;
                        }
                        val => {
                            if !arr.is_empty() {
                                if !have_equiv_types(arr.head().unwrap(), &val) {
                                    debug!("Incompatible element types in array");
                                    return NoValue;
                                }
                            }
                            arr.push(val);
                        }
                    }

                    self.skip_whitespaces_and_comments();
                    if !self.advance_if(',') { break }
                }
                self.skip_whitespaces_and_comments();
                if self.advance_if(']') {
                    return Array(arr);
                } else {
                    return NoValue;
                }
            }
            '"' => {
                match self.parse_string() {
                    Some(str) => { return String(str) }
                    None => { return NoValue }
                }
            }
            _ => { return NoValue }
        }
    }

    fn parse_string(&mut self) -> Option<~str> {
        if !self.advance_if('"') { return None }

        let mut str = StrBuf::new();
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
                                    let ch = (((((d1 as u32 << 4) | d2 as u32) << 4) | d3 as u32) << 4) | d4 as u32;
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
                    return Some(str.into_owned());
                }
                c => {
                    str.push_char(c);
                    self.advance();
                }
            }
        }
    }

    fn read_token(&mut self, f: |char| -> bool) -> ~str {
        let mut token = StrBuf::new();
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

        return token.into_owned();
    }

    fn parse_section_identifier(&mut self) -> ~str {
        self.read_token(|ch| {
            match ch {
                '\t' | '\n' | '\r' | '[' | ']' => false,
                _ => true
            }
        })
    }

    fn skip_whitespaces(&mut self) {
        loop {
            match self.ch() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('\n') => {
                    self.advance();
                    self.line += 1;
                }
                _ => { break }
            }
        }
    }

    fn skip_whitespaces_and_comments(&mut self) {
        loop {
            match self.ch() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('\n') => {
                    self.advance();
                    self.line += 1;
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
        self.line += 1;
        self.advance();
    }

    fn parse<V: Visitor>(&mut self, visitor: &mut V) -> Result<(),Error> {
        loop {
            self.skip_whitespaces_and_comments();

            if self.eos() {
                return self.to_err().map_or(Ok(()), |e| Err(IOError(e)));
            }

            match self.ch().unwrap() {
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
                    // don"t allow empty section names
                    if section_name.is_empty() { return Err(ParseError) }

                    if !self.advance_if(']') { return Err(ParseError) }
                    if double_section {
                        if !self.advance_if(']') { return Err(ParseError) }
                    }

                    if !visitor.section(section_name, double_section) {
                        return Err(ParseError)
                    }
                }

                // identifier: anything else starts an idenfifier!
                // NOTE that we do not allow '.' in identifiers!
                _ => {
                    let ident = self.read_token(|ch| {
                        match ch {
                            ' ' | '\t' | '\r' | '\n' | '=' => false,
                            _ => true
                        }
                    });

                    self.skip_whitespaces();

                    if !self.advance_if('=') { return Err(ParseError) } // assign wanted

                    match self.parse_value() {
                        NoValue => { return Err(ParseError); }
                        val => {
                            if !visitor.pair(ident, val) { return Err(ParseError); }
                        }
                    }
                }
            } /* end match */
        }
    }
}


pub fn parse_from_path(path: &Path) -> Result<Value,Error> {
    let file = File::open(path);
    let mut rd = BufferedReader::new(file);
    return parse_from_buffer(&mut rd);
}

pub fn parse_from_file(name: &str) -> Result<Value,Error> {
    parse_from_path(&Path::new(name))
}

pub fn parse_from_buffer<BUF: Buffer>(rd: &mut BUF) -> Result<Value,Error> {
    let mut ht = ~HashMap::<~str, Value>::new();
    {
        let mut builder = ValueBuilder::new(&mut ht);
        let mut parser = Parser::new(rd);

        match parser.parse(&mut builder) {
            Err(e) => {
                debug!("Error in line: {}", parser.get_line());
                return Err(e);
            }
            Ok(_) => ()
        }
    }
    return Ok(Table(false, ht));
}

pub fn parse_from_bytes(bytes: &[u8]) -> Result<Value,Error> {
    let mut rd = BufReader::new(bytes);
    return parse_from_buffer(&mut rd);
}

pub enum State {
    No,
    Arr(MoveItems<Value>),
    Tab(~HashMap<~str, Value>),
    Map(MoveEntries<~str, Value>)
}

pub struct Decoder {
    value: Value,
    state: State
}

impl Decoder {
    pub fn new(value: Value) -> Decoder {
        Decoder {value: value, state: No}
    }
    pub fn new_state(state: State) -> Decoder {
        Decoder {value: NoValue, state: state}
    }
}

impl serialize::Decoder<Error> for Decoder {
    fn read_nil(&mut self) -> Result<(), Error> { fail!() }

    fn read_u64(&mut self) -> Result<u64, Error> {
        match self.value {
            PosInt(v) => Ok(v),
            _ => fail!()
        }
    }

    fn read_uint(&mut self) -> Result<uint, Error> {
        match self.read_u64() {
            Ok(v) => Ok(v.to_uint().unwrap()),
            Err(_) => fail!()
        }
    }

    fn read_u32(&mut self) -> Result<u32, Error> {
        match self.read_u64() {
            Ok(v) => Ok(v.to_u32().unwrap()),
            Err(_) => fail!()
        }
    }

    fn read_u16(&mut self) -> Result<u16, Error> {
        match self.read_u64() {
            Ok(v) => Ok(v.to_u16().unwrap()),
            Err(_) => fail!()
        }
    }

    fn read_u8(&mut self) -> Result<u8, Error> {
        match self.read_u64() {
            Ok(v) => Ok(v.to_u8().unwrap()),
            Err(_) => fail!()
        }
    }

    fn read_i64(&mut self) -> Result<i64, Error> {
        match self.value {
            PosInt(v) => Ok(v.to_i64().unwrap()),
            NegInt(v) => Ok(-(v.to_i64().unwrap())),
            _ => fail!()
        }
    }

    fn read_int(&mut self) -> Result<int, Error> {
        match self.read_i64() {
            Ok(v) => Ok(v.to_int().unwrap()),
            Err(_) => fail!()
        }
    }

    fn read_i32(&mut self) -> Result<i32, Error> {
        match self.read_i64() {
            Ok(v) => Ok(v.to_i32().unwrap()),
            Err(_) => fail!()
        }
    }

    fn read_i16(&mut self) -> Result<i16, Error> {
        match self.read_u64() {
            Ok(v) => Ok(v.to_i16().unwrap()),
            Err(_) => fail!()
        }
    }

    fn read_i8(&mut self) -> Result<i8, Error> {
        match self.read_u64() {
            Ok(v) => Ok(v.to_i8().unwrap()),
            Err(_) => fail!()
        }
    }


    fn read_bool(&mut self) -> Result<bool, Error> {
        match self.value {
            Boolean(b) => Ok(b),
            _ => fail!()
        }
    }

    fn read_f64(&mut self) -> Result<f64, Error> {
         match self.value {
            Float(f) => Ok(f),
            _ => fail!()
        }
    }

    fn read_f32(&mut self) -> Result<f32, Error> {
        match self.read_f64() {
            Ok(v) => Ok(v.to_f32().unwrap()),
            Err(_) => fail!()
        }
    }

    fn read_char(&mut self) -> Result<char, Error> {
        match self.read_str() {
            Ok(ref s) if (*s).chars().len() == 0 => {
                fail!("no character")
            },
            Err(_) => fail!(),
            Ok(s) => Ok(s[0] as char)
        }
    }

    fn read_str(&mut self) -> Result<~str, Error> {
        match mem::replace(&mut self.value, NoValue) {
            String(s) => Ok(s),
            _ => fail!()
        }
    }

    fn read_enum<T>(&mut self, _name: &str, _f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> { fail!() }
    fn read_enum_variant<T>(&mut self, _names: &[&str], _f: |&mut Decoder, uint| -> Result<T, Error>) -> Result<T, Error> { fail!() }
    fn read_enum_variant_arg<T>(&mut self, _idx: uint, _f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> { fail!() }

    fn read_seq<T>(&mut self, f: |&mut Decoder, uint| -> Result<T, Error>) -> Result<T, Error> {
        match mem::replace(&mut self.value, NoValue) {
            Array(a) | TableArray(a) => {
                let l = a.len();
                f(&mut Decoder::new_state(Arr(a.move_iter())), l)
            }
            _ => fail!()
        }
    }

    fn read_seq_elt<T>(&mut self, _idx: uint, f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> {
        // XXX: assert(idx)
        // XXX: assert!(self.value == NoValue);
        // XXX: self.value = ...
        match self.state {
            Arr(ref mut a) => f(&mut Decoder::new(a.next().unwrap())),
            _ => fail!()
        }
    }

    fn read_struct<T>(&mut self, _name: &str, _len: uint, f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> {
        match mem::replace(&mut self.value, NoValue) {
            Table(_, hm) => {
                f(&mut Decoder::new_state(Tab(hm)))
            }
            _ => fail!()
        }
    }

    fn read_struct_field<T>(&mut self, name: &str, _idx: uint, f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> {
        // XXX: assert!(self.value == NoValue);
        match self.state {
            Tab(ref mut tab) => {
                match tab.pop(&name.to_owned()) { // XXX: pop_equiv(...) or find_equiv_mut...
                    None => f(&mut Decoder::new(NoValue)), // XXX: NoValue means "nil" here
                    Some(val) => f(&mut Decoder::new(val))
                }
            }
            _ => fail!()
        }
    }

    fn read_option<T>(&mut self, f: |&mut Decoder, bool| -> Result<T, Error>) -> Result<T, Error> {
        match self.value {
            NoValue => f(self, false), // XXX
            _ => f(self, true)
        }
    }

    fn read_map<T>(&mut self, f: |&mut Decoder, uint| -> Result<T, Error>) -> Result<T, Error> {
        match mem::replace(&mut self.value, NoValue) {
            Table(_, hm) => {
                let len = hm.len();
                f(&mut Decoder::new_state(Map(hm.move_iter())), len)
            }
            _ => fail!()
        }
    }

    fn read_map_elt_key<T>(&mut self, _idx: uint, f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> {
        let (k, v) = match self.state {
            Map(ref mut map) => {
                match map.next() {
                    None => fail!(),
                    Some((k, v)) => (k, v)
                }
            }
            _ => fail!()
        };
        self.value = String(k);
        let res = f(self);
        self.value = v;
        res
    }

    fn read_map_elt_val<T>(&mut self, _idx: uint, f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> {
        f(self)
    }

    fn read_enum_struct_variant<T>(&mut self,
                                   names: &[&str],
                                   f: |&mut Decoder, uint| -> Result<T, Error>)
                                   -> Result<T, Error> {
        self.read_enum_variant(names, f)
    }


    fn read_enum_struct_variant_field<T>(&mut self,
                                         _name: &str,
                                         idx: uint,
                                         f: |&mut Decoder| -> Result<T, Error>)
                                         -> Result<T, Error> {
        self.read_enum_variant_arg(idx, f)
    }

    fn read_tuple<T>(&mut self, f: |&mut Decoder, uint| -> Result<T, Error>) -> Result<T, Error> {
        self.read_seq(f)
    }

    fn read_tuple_arg<T>(&mut self, idx: uint, f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> {
        self.read_seq_elt(idx, f)
    }

    fn read_tuple_struct<T>(&mut self,
                            _name: &str,
                            f: |&mut Decoder, uint| -> Result<T, Error>)
                            -> Result<T, Error> {
        self.read_tuple(f)
    }

    fn read_tuple_struct_arg<T>(&mut self,
                                idx: uint,
                                f: |&mut Decoder| -> Result<T, Error>)
                                -> Result<T, Error> {
        self.read_tuple_arg(idx, f)
    }
}

pub fn from_toml<'a, T: Decodable<Decoder, Error>>(value: Value) -> T {
    let mut decoder = Decoder::new(value);
    match Decodable::decode(&mut decoder) {
       Ok(config) => config,
       _ => fail!()
    }
}
