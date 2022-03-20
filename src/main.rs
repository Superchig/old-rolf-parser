use core::fmt;
use std::{error::Error, ops};

type ParseResult<T> = std::result::Result<T, ParseError>;
type Result<T> = ParseResult<T>;

fn main() {
    let conf_str = "map ctrl+b";

    let mut scanner = Scanner::new(conf_str);

    println!("{}: {:?}", conf_str, parse(&mut scanner));
    println!("map alt: {:?}", parse(&mut Scanner::new("map alt")));
    println!("map shift: {:?}", parse(&mut Scanner::new("map shift")));
    println!(
        "map shift left: {:?}",
        parse(&mut Scanner::new("map shift left"))
    );
    println!("map lemon: {:?}", parse(&mut Scanner::new("map lemon")));

    println!();

    println!("take_id a: {:?}", Scanner::new("a").take_id());

    test_lex("0");
    test_lex("a");
    test_lex("-");
    test_lex("abra");
}

fn test_lex(input: &str) {
    println!("{}: {:?}", input, lex(&mut Scanner::new(input)));
}

fn lex(scanner: &mut Scanner) -> Result<Vec<Token>> {
    // let lexers: Vec<&dyn Fn(&mut Scanner) -> Result<Token>> = vec![&lex_digit, &lex_letter];
    let lexers: Vec<&dyn Fn(&mut Scanner) -> Result<Token>> = vec![&lex_id];

    let mut tokens = vec![];

    'scanner: while !scanner.is_done() {
        for lexer in &lexers {
            if let Ok(token) = lexer(scanner) {
                tokens.push(token);
                continue 'scanner;
            }
        }

        return Err(ParseError::Message("Failed to finish lexing.".to_string()));
    }

    Ok(tokens)
}

fn lex_id(scanner: &mut Scanner) -> Result<Token> {
    scanner.take_id().map(Token::Id)
}

fn lex_digit(scanner: &mut Scanner) -> Result<Token> {
    match scanner.pop_in_range('0'..='9') {
        Some(ch) => Ok(Token::Digit(ch)),
        None => Err(ParseError::ExpectedDigit),
    }
}

fn lex_letter(scanner: &mut Scanner) -> Result<Token> {
    let lowercase = scanner.pop_in_range('a'..='z');

    if let Some(letter) = lowercase {
        return Ok(Token::Letter(letter));
    }

    let uppercase = scanner.pop_in_range('a'..='Z');

    if let Some(letter) = uppercase {
        return Ok(Token::Letter(letter));
    }

    Err(ParseError::ExpectedLetter)
}

#[derive(Debug)]
enum Token {
    Letter(char),
    Digit(char),
    Id(String),
}

fn parse(scanner: &mut Scanner) -> Result<Map> {
    let result = parse_map(scanner)?;

    if scanner.is_done() {
        Ok(result)
    } else {
        Err(ParseError::from("Input continues beyond map"))
    }
}

fn parse_map(scanner: &mut Scanner) -> Result<Map> {
    scanner.expect_str("map ")?;

    let key = parse_key(scanner)?;

    Ok(Map { key })
}

fn parse_key(scanner: &mut Scanner) -> Result<Key> {
    match parse_mod(scanner) {
        Ok(mod_enum) => {
            // FIXME(Chris): Implement custom errors, with display of line and column number
            scanner.expect(&'+')?;

            // TODO(Chris): Implement support for RangeInclusive
            let key_char = scanner
                .pop_in_range('a'..='z')
                .ok_or("Failed to find letter")?;

            Ok(Key {
                key_char,
                modifier: Some(mod_enum),
            })
        }
        Err(_) => {
            let key_char = scanner
                .pop_in_range('a'..='z')
                .ok_or("Failed to find letter")?;

            Ok(Key {
                key_char,
                modifier: None,
            })
        }
    }
}

fn parse_mod(scanner: &mut Scanner) -> Result<Mod> {
    if scanner.take_str("ctrl") {
        Ok(Mod::Ctrl)
    } else if scanner.take_str("shift") {
        Ok(Mod::Shift)
    } else if scanner.take_str("alt") {
        Ok(Mod::Alt)
    } else {
        Err(ParseError::from(
            "A modifier requires a ctrl, shift, or alt",
        ))
    }
}

#[derive(Debug)]
struct Map {
    key: Key,
}

#[derive(Debug)]
struct Key {
    modifier: Option<Mod>,
    key_char: char,
}

#[derive(Debug)]
enum Mod {
    Ctrl,
    Shift,
    Alt,
}

pub struct Scanner {
    cursor: usize,
    characters: Vec<char>,
}

impl Scanner {
    pub fn new(string: &str) -> Self {
        Self {
            cursor: 0,
            characters: string.chars().collect(),
        }
    }

    /// Returns the current cursor. Useful for reporting errors.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the next character without advancing the cursor.
    /// AKA "lookahead"
    pub fn peek(&self) -> Option<&char> {
        self.characters.get(self.cursor)
    }

    /// Returns true if further progress is not possible
    pub fn is_done(&self) -> bool {
        self.cursor >= self.characters.len()
    }

    /// Returns the next character (if available) and advances the cursor.
    pub fn pop(&mut self) -> Option<&char> {
        match self.characters.get(self.cursor) {
            Some(character) => {
                self.cursor += 1;

                Some(character)
            }
            None => None,
        }
    }

    /// Returns the next character if it's in the given range, and advances the cursor.
    /// Otherwise, returns None, leaving the cursor unchanged.
    pub fn pop_in_range(&mut self, target_range: ops::RangeInclusive<char>) -> Option<char> {
        match self.peek() {
            Some(ch) => {
                if target_range.contains(ch) {
                    let copy = *ch;

                    self.pop();

                    Some(copy)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    /// Returns true if the `target` is found at the current cursor position,
    /// and advances the cursor.
    /// Otherwise, returns false, leaving the cursor unchanged.
    pub fn take(&mut self, target: &char) -> bool {
        match self.characters.get(self.cursor) {
            Some(character) => {
                if target == character {
                    self.cursor += 1;

                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }

    /// Returns Some(()) if the `target` is found at the current cursor position, and advances the
    /// cursor.
    /// Otherwise, returns None, leaving the cursor unchanged.
    pub fn expect(&mut self, target: &char) -> ParseResult<()> {
        match self.characters.get(self.cursor) {
            Some(character) => {
                if target == character {
                    self.cursor += 1;

                    Ok(())
                } else {
                    Err(ParseError::Expected(*target))
                }
            }
            None => Err(ParseError::Expected(*target)),
        }
    }

    pub fn take_str(&mut self, target: &str) -> bool {
        if target.len() + self.cursor > self.characters.len() {
            return false;
        }

        let mut ind = self.cursor;

        for ch in target.chars() {
            if ch != self.characters[ind] {
                return false;
            }

            ind += 1;
        }

        self.cursor = ind;

        true
    }

    pub fn expect_str(&mut self, target: &str) -> ParseResult<()> {
        if self.take_str(target) {
            Ok(())
        } else {
            Err(ParseError::ExpectedPhrase(target.to_string()))
        }
    }

    /// Invoke `cb` once. If the result is not `None`, return it and advance
    /// the cursor. Otherwise, return None and leave the cursor unchanged.
    pub fn transform<T>(&mut self, cb: impl FnOnce(&char) -> Option<T>) -> Option<T> {
        match self.characters.get(self.cursor) {
            Some(input) => match cb(input) {
                Some(output) => {
                    self.cursor += 1;

                    Some(output)
                }
                None => None,
            },
            None => None,
        }
    }

    pub fn take_id(&mut self) -> ParseResult<String> {
        let mut curr_index = self.cursor;
        let mut buf = String::new();

        while curr_index < buf.len() {
            let ch = self.characters[curr_index];

            if ('a'..='z').contains(&ch) || ('A'..='Z').contains(&ch) {
                buf.push(ch);
            } else {
                break;
            }

            curr_index += 1;
        }

        if buf.is_empty() {
            return Err(ParseError::ExpectedId);
        }

        self.cursor = curr_index;

        Ok(buf)
    }
}

#[derive(Debug)]
pub enum ParseError {
    Message(String),
    Expected(char),
    ExpectedPhrase(String),
    ExpectedDigit,
    ExpectedLetter,
    ExpectedId,
}

impl From<&str> for ParseError {
    fn from(message: &str) -> Self {
        ParseError::Message(message.to_string())
    }
}

impl From<String> for ParseError {
    fn from(message: String) -> Self {
        ParseError::Message(message)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ParseError {}
