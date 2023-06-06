mod lexer;
mod parser;

// Type definition for the Result that is being used by the parser. You may change it to anything
// you want
pub type ParseResult = Result<(), String>;

pub use lexer::C1Lexer;
pub use lexer::C1Token;
pub use parser::C1Parser;

