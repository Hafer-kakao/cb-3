use logos::{Lexer, Logos};

#[derive(Logos, Debug, PartialEq, Copy, Clone)]
pub enum C1Token {
    #[token("bool")]
    KwBoolean,

    #[token("do")]
    KwDo,

    #[token("else")]
    KwElse,

    #[token("float")]
    KwFloat,

    #[token("for")]
    KwFor,

    #[token("if")]
    KwIf,

    #[token("int")]
    KwInt,

    #[token("printf")]
    KwPrintf,

    #[token("return")]
    KwReturn,

    #[token("void")]
    KwVoid,

    #[token("while")]
    KwWhile,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Asterisk,

    #[token("/")]
    Slash,

    #[token("=")]
    /// =
    Assign,

    #[token("==")]
    /// ==
    Equal,

    #[token("!=")]
    /// !=
    NotEqual,

    #[token("<")]
    /// <
    Less,

    #[token(">")]
    /// >
    Greater,

    #[token("<=")]
    /// <=
    LessEqual,

    #[token(">=")]
    /// >=
    GreaterEqual,

    #[token("&&")]
    /// &&
    And,

    #[token("||")]
    /// ||
    Or,

    #[token(",")]
    Comma,

    #[token(";")]
    Semicolon,

    #[token("(")]
    /// (
    LeftParenthesis,

    #[token(")")]
    /// )
    RightParenthesis,

    #[token("{")]
    /// {
    LeftBrace,

    #[token("}")]
    /// }
    RightBrace,

    #[regex("[0-9]+")]
    ConstInt,

    #[regex(r"(\d+\.\d+)|(\.\d+([eE]([-+])?\d+)?)|(\d+[eE]([-+])?\d+)")]
    ConstFloat,

    #[regex("true|false")]
    ConstBoolean,

    #[regex("\"[^\n\"]*\"")]
    ConstString,

    #[regex("[a-zA-Z]+[0-9a-zA-Z]*")]
    Identifier,

    #[regex(r"/\*[^\*/]*\*/", logos::skip)]
    CComment,

    #[regex("//[^\n]*(\n)?", logos::skip)]
    CPPComment,

    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\f]+", logos::skip)]
    Whitespace,

    #[regex(r"(\r\n|\r|\n)")]
    Linebreak,

    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    Error,
}

pub struct C1Lexer<'a> {
    logos_lexer: Lexer<'a, C1Token>,
    logos_line_number: usize,
    current_token: Option<TokenData<'a>>,
    past: Vec<TokenData<'a>>,
    marks: usize,
    position: usize,
}

impl<'a> C1Lexer<'a> {
    /// Initialize a new C1Lexer for the given string slice
    pub fn new(text: &'a str) -> C1Lexer {
        let mut lexer = C1Lexer {
            logos_lexer: C1Token::lexer(text),
            logos_line_number: 1,
            current_token: None,
            past: vec![],
            marks: 0,
            position: 0,
        };
        lexer.advance();
        lexer
    }

    /// Return the C1Token variant of the current token without consuming it.
    /// ```
    /// use cb_3::{C1Lexer, C1Token};
    /// let lexer = C1Lexer::new("current next");
    ///
    /// assert_eq!(lexer.current_token(), Some(C1Token::Identifier));
    /// assert_eq!(lexer.current_text(), Some("current"));
    ///
    /// assert_eq!(lexer.current_token(), Some(C1Token::Identifier));
    /// assert_eq!(lexer.current_text(), Some("current"));
    /// ```
    pub fn current_token(&self) -> Option<C1Token> {
        self.current_token.token_type()
    }

    /// Return the text of the current token
    pub fn current_text(&self) -> Option<&str> {
        self.current_token.text()
    }


    /// Return the line number where the current token is located
    pub fn current_line_number(&self) -> Option<usize> {
        self.current_token.line_number()
    }

    pub fn mark(&mut self) -> usize {
        self.marks += 1;
        if self.marks == 1 {
            if let Some(t) = self.current_token {
                self.past = vec!(t);
                self.position = 1;
            }
        } 
        self.position-1
    }

    pub fn undo(&mut self, marker: usize) {
        self.position = marker;
        self.advance();
    }

    pub fn pop_mark(&mut self) {
        self.marks -= 1;
    }

    pub fn advance(&mut self)  {
        let next;
        if self.position >= self.past.len() {
            next = self.next_token_lexer();
        } else {
            next = Some(self.past[self.position]);
            self.position += 1;
        }

        if self.marks == 0 && self.position >= self.past.len() && self.past.len() != 0 {
            self.past = vec![];
            self.position = 0;
        }

        self.current_token = next;
    }

    fn next_token_lexer(&mut self) -> Option<TokenData<'a>> {
        if let Some(c1_token) = self.logos_lexer.next() {
            match c1_token {
                C1Token::Linebreak => {
                    self.logos_line_number += 1;
                    self.next_token_lexer()
                }
                _ => {
                    let next= TokenData {
                        token_type: c1_token,
                        token_text: self.logos_lexer.slice(),
                        token_line: self.logos_line_number
                    };
                    
                    if self.marks > 0 {
                        self.past.push(next);
                    }
                    self.position = self.past.len();
                    Some(next)
                },
            }
        } else {
            self.position = self.past.len() + 1;
            None
        }
    }

}

/// Hidden struct for capsuling the data associated with a token.
#[derive(Copy, Clone, Debug)]
pub struct TokenData<'a> {
    token_type: C1Token,
    token_text: &'a str,
    token_line: usize,
}

/// Hidden trait that makes it possible to implemented the required getter functionality directly for
/// Option<TokenData>.
trait TokenDataProvider<'a> {
    /// Return the type of the token, aka. its C1Token variant.
    fn token_type(&self) -> Option<C1Token>;
    /// Return the text of the token
    fn text(&self) -> Option<&str>;
    /// Return the line number of the token
    fn line_number(&self) -> Option<usize>;
}

impl<'a> TokenDataProvider<'a> for Option<TokenData<'a>> {
    fn token_type(&self) -> Option<C1Token> {
        self.as_ref().map(|data| data.token_type)
    }

    fn text(&self) -> Option<&'a str> {
        self.as_ref().map(|data| data.token_text)
    }

    fn line_number(&self) -> Option<usize> {
        self.as_ref().map(|data| data.token_line)
    }
}