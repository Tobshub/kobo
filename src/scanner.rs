use crate::token::{keyword, Token, TokenType, one_line};

/// Cut `source` into tokens. Returns everything it managed to scan alongside every
/// error it found; the caller decides whether to go on.
pub fn scan(source: &str) -> (Vec<Token>, Vec<String>) {
    let mut s = Scanner {
        src: source.chars().collect(),
        start: 0,
        current: 0,
        line: 1,
        tokens: Vec::new(),
        errors: Vec::new(),
    };
    s.run();
    (s.tokens, s.errors)
}

struct Scanner {
    src: Vec<char>,
    start: usize,
    current: usize,
    line: usize,
    tokens: Vec<Token>,
    errors: Vec<String>,
}

impl Scanner {
    fn run(&mut self) {
        // TODO(you): drive the scan: read one token at a time until the source runs out, then
        //            add the EOF token. Spec 6.1 says which line EOF carries.
        // todo!("run")
        while !self.at_end() {
            self.scan_token();
        }
        self.add(TokenType::Eof);
    }

    fn scan_token(&mut self) {
        // TODO(you): recognise one token. Spec 1.2 lists every token type, 1.1 covers
        //            whitespace and comments, and an unrecognised character is 'Character is
        //            not part of any token.' (5.1).
        // todo!("scan_token")
        // println!("top level peek: \"{}\"", one_line(self.peek().to_string().as_str()));
        match self.advance() {
            // whitespace
            ' ' => (),
            '\t' => (),
            '\r' => (),
            '\n' => self.line += 1,

            '(' => self.add(TokenType::LParen),
            ')' => self.add(TokenType::RParen),
            '{' => self.add(TokenType::LBrace),
            '}' => self.add(TokenType::RBrace),
            ',' => self.add(TokenType::Comma),
            ';' => self.add(TokenType::Semicolon),

            '+' => self.add(TokenType::Plus),
            '-' => self.add(TokenType::Minus),
            '*' => self.add(TokenType::Star),
            '/' => {
                if self.peek() == '/' {
                    while self.peek() != '\n' && !self.at_end() {
                        self.advance();
                    };
                    // don't advance here, let's next scan_token call handle newlines
                    // self.line += 1;
                    // self.advance();
                    // let lexeme: String = self.src[self.start..self.current].iter().collect();
                    // println!("{}", one_line(self.peek().to_string().as_str()));
                    // println!("{}", one_line(lexeme.as_str()));
                } else {
                    self.add(TokenType::Slash);
                }
            },

            '!' => {
                if self.peek() == '=' {
                    self.advance();
                    self.add(TokenType::BangEqual);
                } else {
                    self.add(TokenType::Bang);
                }
            },
            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    self.add(TokenType::EqualEqual);
                } else {
                    self.add(TokenType::Equal);
                }
            },
            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    self.add(TokenType::GreaterEqual);
                } else {
                    self.add(TokenType::Greater);
                }
            },
            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    self.add(TokenType::LessEqual);
                } else {
                    self.add(TokenType::Less);
                }
            },

            '"' => self.string(),
            '0'..='9' => self.number(),
            '_' | 'a'..='z' | 'A'..='Z' => self.identifier(),

            _ => self.error(self.line, "Character is not part of any token."),

        }

        self.start = self.current;
    }

    fn string(&mut self) {
        // TODO(you): scan a string literal. A string may span lines (1.5); an unterminated one
        //            is reported at the line it opened on (5.1).
        // todo!("string")
        // while self.advance() != '"' {
        //     if self.at_end() { self.line += 1; break; }
        //     if self.peek() == '\n' { self.line += 1; }
        // }
        let mut terminated = false;
        let mut line_offset = 0; // keep track of line offset separately in case of unterminated string error reporting
        while !self.at_end() && !terminated {
            match self.peek() {
                '"' => { terminated = true; },
                '\n' => line_offset += 1,
                _ => (),
            };
            self.advance();
        }
        if !terminated {
            self.error(self.line, "String is never closed.");
            return;
        }
        self.line += line_offset;
        self.add(TokenType::Str);
    }

    fn number(&mut self) {
        // TODO(you): scan a number literal: digits, then a fractional part only when a digit
        //            follows the dot (1.4).
        // todo!("number")
        let mut has_dot = false;
        while !self.at_end() {
            match self.peek() {
                '.' => {
                    match self.peek_next() {
                        '0'..='9' => {
                            if has_dot {
                                self.error(self.line, "Character is not part of any token."); return;
                            } else { has_dot = true; }
                            self.advance();
                        }
                        _ => { self.error(self.line, "Character is not part of any token.");  return; }
                    }
                },
                '0'..='9' => { self.advance(); },
                _ => break,
            };
        }
        self.add(TokenType::Number);
    }

    fn identifier(&mut self) {
        // TODO(you): scan an identifier, then decide whether it is a keyword; keyword() in
        //            token.rs does the lookup (1.2, 1.3).
        // todo!("identifier")
        while !self.at_end() {
            match self.peek() {
                '_' | 'a'..='z' | 'A'..='Z' | '0'..='9' =>   { self.advance(); },
                _ => break,
            }
        }
        let lexeme: String = self.src[self.start..self.current].iter().collect();
        match keyword(lexeme.as_str()) {
            Some(tt) => self.add(tt),
            None => self.add(TokenType::Identifier),
        }
    }

    // --- primitives ---------------------------------------------------------------

    fn at_end(&self) -> bool {
        self.current >= self.src.len()
    }

    fn advance(&mut self) -> char {
        let c = self.src[self.current];
        self.current += 1;
        c
    }

    fn matches(&mut self, expected: char) -> bool {
        if self.at_end() || self.src[self.current] != expected {
            return false;
        }
        self.current += 1;
        true
    }

    fn peek(&self) -> char {
        if self.at_end() {
            '\0'
        } else {
            self.src[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.src.len() {
            '\0'
        } else {
            self.src[self.current + 1]
        }
    }

    fn add(&mut self, kind: TokenType) {
        if kind == TokenType::Eof {
            self.line = match self.tokens.last() {
                None => 0,
                Some(t) => t.line,
            };
        }
        let lexeme = self.src[self.start..self.current].iter().collect();
        // println!("\"{}\"", lexeme);
        self.tokens.push(Token {
            kind,
            lexeme,
            line: self.line,
        });
    }

    fn error(&mut self, line: usize, message: &str) {
        self.errors.push(format!("[line {}] Error: {}", line, message));
    }
}
