use std::collections::{HashMap, BTreeMap};
use std::fmt::{self, Write, Formatter, Debug};
use std::str::{FromStr, from_utf8};
use std::ops::{Index, Deref};
use std::borrow::Borrow;
use std::iter::FromIterator;
use std::char::{decode_utf16, DecodeUtf16Error};
use std::mem::{MaybeUninit, ManuallyDrop};
use std::convert::TryFrom;
use rand::distributions::Open01;

#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Boolean(bool),
    Number(f64),
    String(JsonString),
    Array(Vec<Json>),
    Object(Object),
}

impl Json {
    fn get_null(&self) -> Option<()> {
        match *self {
            Json::Null => Some(()),
            _ => None,
        }
    }
    fn get_bool(&self) -> Option<bool> {
        match *self {
            Json::Boolean(b) => Some(b),
            _ => None,
        }
    }
    fn get_number(&self) -> Option<f64> {
        match *self {
            Json::Number(n) => Some(n),
            _ => None,
        }
    }
    pub fn get_string(&self) -> Option<&str> {
        match *self {
            Json::String(ref s) => Some(s.as_str()),
            _ => None,
        }
    }
    fn get_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(a) => Some(a.as_slice()),
            _ => None,
        }
    }
    fn get_object(&self) -> Option<&Object> {
        match self {
            Json::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn null(&self) {
        self.get_null().unwrap()
    }

    pub fn bool(&self) -> bool {
        self.get_bool().unwrap()
    }
    
    pub fn number(&self) -> f64 {
        self.get_number().unwrap()
    }

    pub fn string(&self) -> &str {
        self.get_string().unwrap()
    }

    pub fn array(&self) -> &[Json] {
        self.get_array().unwrap()
    }

    pub fn object(&self, key: &str) -> &Json {
        &self.get_object().unwrap()[key]
    }
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Json::Null => f.write_str("null"),
            Json::Boolean(b) => fmt::Display::fmt(b, f),
            Json::Number(n) => fmt::Display::fmt(n, f),
            Json::String(ref s) => write_json_string(s, f),
            Json::Array(ref a) => {
                f.write_str("[")?;
                let mut comma = "";
                for elem in a {
                    f.write_str(comma)?;
                    fmt::Display::fmt(elem, f)?;
                    comma = ",";
                }
                f.write_str("]")
            },
            Json::Object(ref m) => {
                f.write_str("{")?;
                let mut comma = "";
                for (k, v) in m.iter() {
                    f.write_str(comma)?;
                    write_json_string(k, f)?;
                    f.write_str(":")?;
                    fmt::Display::fmt(v, f)?;
                    comma = ",";
                }
                f.write_str("}")
            },
        }
    }
}

impl FromStr for Json {
    type Err = ();

    fn from_str(s: &str) -> Result<Json, ()> {
        let s = s.trim();

        if let "null" = s {
            Ok(Json::Null)
        } else if let Ok(b) = s.parse::<bool>() {
            Ok(Json::Boolean(b))
        } else if let Ok(n) = s.parse::<f64>() {
            Ok(Json::Number(n.into()))
        } else if let Ok(ret) = parse_json_string(s) {
            Ok(Json::String(ret))
        } else if s.starts_with('[') && s.ends_with(']') {
            Ok(Json::Array(SplitTopLevel::new(&s[1..s.len()-1], b',')
                .filter(|value| !value.chars().all(char::is_whitespace))
                .map(|value| value.parse())
                .collect::<Result<Vec<Json>, ()>>()?
            ))

        } else if s.starts_with('{') && s.ends_with('}') {
            Ok(Json::Object(SplitTopLevel::new(&s[1..s.len()-1], b',')
                .map(|keypair| {
                    let mut a = SplitTopLevel::new(keypair, b':');
                    let key = a.next().ok_or(())?;
                    let value = a.next().ok_or(())?;
                    Ok((parse_json_string(key.trim())?, value.parse()?))
                })
                .collect::<Result<Object, ()>>()?
            ))

        } else {
            Err(())
        }
    }
}

struct SplitTopLevel<'a> {
    bytes: &'a [u8],
    split_on: u8,
}

impl<'a> SplitTopLevel<'a> {
    fn new(s: &'a str, split_on: u8) -> SplitTopLevel<'a> {
        assert!(split_on.is_ascii());
        SplitTopLevel {
            bytes: s.as_bytes(),
            split_on
        }
    }
}

impl<'a> Iterator for SplitTopLevel<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.bytes.is_empty() { return None }
        let mut bracket_count = 0;
        let mut mustache_count = 0;
        let mut quote_count_even = true;
        let mut char_is_escaped = false;

        for (i, &b) in self.bytes.iter().enumerate() {
            if !char_is_escaped {
                match b {
                    b'[' if quote_count_even => bracket_count += 1,
                    b']' if quote_count_even => bracket_count -= 1,
                    b'{' if quote_count_even => mustache_count += 1,
                    b'}' if quote_count_even => mustache_count -= 1,
                    b'"' => quote_count_even = !quote_count_even,
                    _ if b == self.split_on && quote_count_even && bracket_count == 0 && mustache_count == 0 => {
                        let ret = from_utf8(&self.bytes[..i]).unwrap();
                        self.bytes = &self.bytes[i+1..];
                        return Some(ret)
                    },
                    _ => {},
                }
            }

            char_is_escaped = b == b'\\' && !char_is_escaped;
        }

        if bracket_count == 0 && mustache_count == 0 && quote_count_even {
            let ret = from_utf8(self.bytes).unwrap(); // we were passed in valid utf8
            self.bytes = &[];
            Some(ret)
        } else {
            self.bytes = &[];
            None
        }
    }
}

fn write_json_string(mut string: &str, f: &mut fmt::Formatter) -> fmt::Result {
    fn escape_needed(c: u8) -> bool {
        c < b' ' || c > b'~' || c == b'"' || c == b'\\'
    }

    fn partition_by_escape(string: &str) -> (&str, &str) {
        let len = string.bytes().position(escape_needed).unwrap_or(string.len());
        string.split_at(len)
    }

    f.write_char('"')?;

    loop {
        let (no_escape_prefix, new_string) = partition_by_escape(string);
        f.write_str(no_escape_prefix)?;
        string = new_string;

        let mut chars = string.chars();

        match chars.next() {
            None => break,
            Some('"') => f.write_str("\\\"")?,
            Some('\\') => f.write_str("\\\\")?,
            Some('\x08') => f.write_str("\\b")?,
            Some('\x0c') => f.write_str("\\f")?,
            Some('\n') => f.write_str("\\n")?,
            Some('\r') => f.write_str("\\r")?,
            Some('\t') => f.write_str("\\t")?,
            Some(c) => {
                for h in c.encode_utf16(&mut [0; 2]) {
                    write!(f, "\\u{:04X}", h)?;
                }
            }
        }

        string = chars.as_str();
    }

    f.write_char('"')
}

fn parse_json_string(s: &str) -> Result<JsonString, ()> {
    if s.len() < 2 || !s.starts_with("\"") || !s.ends_with("\"") { return Err(()) }

    let mut ret = JsonString::with_capacity(s.len());
    let mut chars = s[1..s.len()-1].chars();

    loop {
        let c = match chars.next() {
            Some(c) => c,
            None => break Ok(ret),
        };

        match c {
            '\\' => match chars.next().ok_or(())? {
                '"' => ret.push('"'),
                '\\' => ret.push('\\'),
                '/' => ret.push('/'),
                'b' => ret.push('\x08'),
                'f' => ret.push('\x0c'),
                'n' => ret.push('\n'),
                'r' => ret.push('\r'),
                't' => ret.push('\t'),
                'u' => {
                    let value = &chars.as_str().get(..4).ok_or(())?;
                    let u1 = u16::from_str_radix(value, 16).map_err(|_| ())?;

                    chars.nth(4-1).ok_or(())?; // advance iter by 4 chars

                    match decode_utf16(Some(u1)).next().unwrap() {
                        Ok(c) => ret.push(c),
                        Err(_) => { // we probably need the other surrogate pair
                            if !chars.as_str().starts_with("\\u") { return Err(()) }
                            chars.nth(2-1).ok_or(())?;
                            let u2 = &chars.as_str().get(..4).ok_or(())?;
                            let u2 = u16::from_str_radix(u2, 16).map_err(|_| ())?;
                            let c = decode_utf16([u1, u2].iter().copied()).next().unwrap().map_err(|_| ())?;
                            ret.push(c);
                        },
                    }
                },
                _ => return Err(()),
            },
            '"' => return Err(()),
            _ => ret.push(c),
        }
    }
}



#[derive(Clone, PartialEq, Eq, Hash)]
pub struct JsonString {
    inner: String, // todo: better representation
}

impl JsonString {
    fn with_capacity(capacity: usize) -> JsonString {
        JsonString { inner: String::with_capacity(capacity) }
    }

    fn from_str(string: &str) -> JsonString {
        JsonString { inner: string.to_string() }
    }

    fn push(&mut self, c: char) {
        self.inner.push(c);
    }

    fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl PartialEq<str> for JsonString {
    fn eq(&self, other: &str) -> bool {
        (**self).eq(other)
    }
}

impl From<String> for JsonString {
    fn from(inner: String) -> JsonString {
        JsonString { inner }
    }
}

impl From<&str> for JsonString {
    fn from(string: &str) -> JsonString {
        JsonString::from_str(string)
    }
}

impl Borrow<str> for JsonString {
    fn borrow(&self) -> &str {
        &**self
    }
}

impl Debug for JsonString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_json_string(&*self, f)
    }
}

impl Deref for JsonString {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Clone)]
pub struct Object {
    inner: Vec<(JsonString, Json)>,
    indexes: HashMap<JsonString, usize>,
}

impl Object {
    pub fn new() -> Object {
        Object { inner: Vec::new(), indexes: HashMap::new() }
    }

    fn with_capacity(capacity: usize) -> Object {
        Object {
            inner: Vec::with_capacity(capacity),
            indexes: HashMap::with_capacity(capacity),
        }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn iter(&self) -> impl Iterator<Item=(&str, &Json)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn insert(&mut self, key: &str, value: Json) {
        self.insert_string(JsonString::from_str(key), value);
    }

    fn insert_string(&mut self, key: JsonString, value: Json) {
        let index = self.inner.len();
        self.inner.push((key.clone(), value));
        self.indexes.insert(key, index);
    }

    fn get(&self, key: &str) -> Option<&Json> {
        let index = *self.indexes.get(key)?;
        Some(&self.inner.get(index)?.1)
    }
}

impl Index<&str> for Object {
    type Output = Json;

    fn index(&self, index: &str) -> &Json {
        self.get(index).unwrap()
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'a> FromIterator<(JsonString, Json)> for Object {
    fn from_iter<T: IntoIterator<Item=(JsonString, Json)>>(iter: T) -> Object {
        let iter = iter.into_iter();

        let (capacity, _) = iter.size_hint();
        let mut ret = Object::with_capacity(capacity);

        for (k, v) in iter {
            ret.insert_string(k, v);
        }

        ret
    }
}