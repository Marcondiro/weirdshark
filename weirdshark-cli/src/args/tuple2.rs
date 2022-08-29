use std::fmt::{Display, Formatter};
use std::error::Error;
use std::str::FromStr;
use ParseError::{ElementParsing,MissingBracket,BadValueNumber};

#[derive(Debug)]
pub struct Tuple2<T: FromStr> {
    pub(crate) _0: T,
    pub(crate) _1: T,
}

//TODO:ToBe tested
impl<T: FromStr> FromStr for Tuple2<T> {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s_mut = s.trim();
        if !s_mut.starts_with("(") {
            return Err(MissingBracket(s_mut.to_string()));
        }
        let mut chars = s_mut.chars();
        chars.next();
        if !s_mut.ends_with(")") {
            return Err(MissingBracket(s_mut.to_string()));
        }
        chars.next_back();
        s_mut = chars.as_str();

        let vec: Vec<&str> = s_mut.split(",").map(|s| { s.trim() }).collect();
        if vec.len() != 2 {
            return Err(BadValueNumber(s_mut.to_string()));
        }
        let mut _0 = None;
        let mut _1 = None;
        for (i, string) in vec.iter().enumerate() {
            let el = match T::from_str(string) {
                Ok(el) => el,
                Err(_err) => return Err(ElementParsing(format!("Error parsing one element from \"{}\"", string)))
            };
            if i == 0 {
                _0 = Some(el);
            } else {
                _1 = Some(el);
            }
        }


        Ok(Tuple2 { _0: _0.unwrap(), _1: _1.unwrap() })
    }
}

#[derive(Debug)]
pub enum ParseError {
    GenericError(String),
    BadValueNumber(String),
    MissingBracket(String),
    ElementParsing(String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ParseError {}