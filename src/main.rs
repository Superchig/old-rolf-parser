use core::fmt;
use std::{error::Error, ops, mem};

type ParseResult<T> = std::result::Result<T, ParseError>;
type Result<T> = ParseResult<T>;

fn main() {
    test_lex("ctrl");
    test_lex("a");
    test_lex("-");
    test_lex("abra");
    test_lex("map ctrl");
    test_lex("map ctrl+a");
    test_lex("map ctrl+k up");
}

fn test_lex(input: &str) {
    println!("{}: {:#?}", input, lex(&mut Scanner::new(input)));
}

fn lex(scanner: &mut Scanner) -> Result<Vec<Token>> {
    let lex_map = lex_phrase("map ");
    let lex_plus = lex_phrase("+");

    // NOTE(Chris): The order matters here, in case one lexing rule conflicts with another.
    let mut lexers: Vec<&dyn Fn(&mut Scanner) -> Result<Token>> =
        vec![&lex_mod, &lex_whitespace, &lex_map, &lex_plus];

    lexers.push(&lex_id);

    let mut tokens = vec![];

    'scanner: while !scanner.is_done() {
        for lexer in &lexers {
            if let Ok(token) = lexer(scanner) {
                tokens.push(token);
                continue 'scanner;
            }
        }

        eprintln!("failed tokens: {:#?}", tokens);

        return Err(ParseError::Message("Failed to finish lexing.".to_string()));
    }

    // Move the line and column numbers "back" for each token, so that they contain their starting
    // positions rather than their ending positions.
    let mut prev_line = 1;
    let mut prev_col = 1;
    for token in &mut tokens {
        mem::swap(&mut token.line, &mut prev_line);
        mem::swap(&mut token.col, &mut prev_col);
    }

    Ok(tokens)
}

fn lex_id(scanner: &mut Scanner) -> Result<Token> {
    let mut buf = String::new();

    loop {
        let lowercase = scanner.pop_in_range('a'..='z');

        if let Some(letter) = lowercase {
            buf.push(letter);
            continue;
        }

        let uppercase = scanner.pop_in_range('a'..='z');

        if let Some(letter) = uppercase {
            buf.push(letter);
            continue;
        }

        break;
    }

    if buf.is_empty() {
        Err(ParseError::ExpectedId)
    } else {
        Ok(Token::new(scanner, TokenKind::Id(buf)))
    }
}

fn lex_mod(scanner: &mut Scanner) -> Result<Token> {
    if scanner.take_str("ctrl") {
        Ok(Token::new(scanner, TokenKind::Mod(Mod::Ctrl)))
    } else if scanner.take_str("shift") {
        Ok(Token::new(scanner, TokenKind::Mod(Mod::Shift)))
    } else if scanner.take_str("alt") {
        Ok(Token::new(scanner, TokenKind::Mod(Mod::Alt)))
    } else {
        Err(ParseError::from(
            "A modifier requires a ctrl, shift, or alt",
        ))
    }
}

fn lex_phrase(phrase: &'static str) -> Box<dyn Fn(&mut Scanner) -> Result<Token>> {
    Box::new(move |scanner: &mut Scanner| {
        if scanner.take_str(phrase) {
            Ok(Token::new(scanner, TokenKind::Phrase(phrase)))
        } else {
            Err(ParseError::ExpectedPhrase(phrase))
        }
    })
}

fn lex_whitespace(scanner: &mut Scanner) -> Result<Token> {
    let mut was_whitespace = false;

    while let Some(_ch) = scanner.pop_in_slice(&[' ', '\t']) {
        was_whitespace = true;
    }

    if was_whitespace {
        Ok(Token::new(scanner, TokenKind::Whitespace))
    } else {
        Err(ParseError::ExpectedWhitespace)
    }
}

#[derive(Debug)]
pub struct Token {
    line: usize,
    col: usize,
    kind: TokenKind,
}

impl Token {
    pub fn new(scanner: &Scanner, kind: TokenKind) -> Self {
        Token {
            line: scanner.curr_line,
            col: scanner.curr_col,
            kind,
        }
    }
}

#[derive(Debug)]
pub enum TokenKind {
    Id(String),
    Mod(Mod),
    Phrase(&'static str),
    Whitespace,
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
    // scanner.expect_str("map ")?;

    let key = parse_key(scanner)?;

    Ok(Map { key })
}

fn parse_key(scanner: &mut Scanner) -> Result<Key> {
    match parse_mod(scanner) {
        Ok(mod_enum) => {
            scanner.expect(&'+')?;

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
        Err(ParseError::ExpectedMod)
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
pub enum Mod {
    Ctrl,
    Shift,
    Alt,
}

pub struct Parser {
    cursor: usize,
    tokens: Vec<Token>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            cursor: 0,
            tokens,
        }
    }
}

pub struct Scanner {
    cursor: usize,
    characters: Vec<char>,
    curr_line: usize,
    curr_col: usize,
}

impl Scanner {
    pub fn new(string: &str) -> Self {
        Self {
            cursor: 0,
            characters: string.chars().collect(),
            // Files start at line 1, column 1
            curr_line: 1,
            curr_col: 1,
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
                if character == &'\n' {
                    self.curr_line += 1;
                    self.curr_col = 1;
                } else {
                    self.curr_col += 1;
                }

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

    /// Returns the next character if it's in the given slice, and advances the cursor.
    /// Otherwise, returns None, leaving the cursor unchanged.
    pub fn pop_in_slice(&mut self, range_slice: &[char]) -> Option<char> {
        match self.peek() {
            Some(ch) => {
                if range_slice.contains(ch) {
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
                    self.pop();

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
                    self.pop();

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

        let orig_cursor = self.cursor;

        for _ in orig_cursor..ind {
            self.pop();
        }

        true
    }

    /// Invoke `cb` once. If the result is not `None`, return it and advance
    /// the cursor. Otherwise, return None and leave the cursor unchanged.
    pub fn transform<T>(&mut self, cb: impl FnOnce(&char) -> Option<T>) -> Option<T> {
        match self.characters.get(self.cursor) {
            Some(input) => match cb(input) {
                Some(output) => {
                    self.pop();

                    Some(output)
                }
                None => None,
            },
            None => None,
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    Message(String),
    Expected(char),
    ExpectedPhrase(&'static str),
    ExpectedDigit,
    ExpectedLetter,
    ExpectedId,
    ExpectedMod,
    ExpectedWhitespace,
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
