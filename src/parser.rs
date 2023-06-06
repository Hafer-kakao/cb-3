use crate::lexer::{C1Lexer, C1Token};
use crate::ParseResult;
use std::ops::{Deref, DerefMut};

pub struct C1Parser<'a>(C1Lexer<'a>);
// Implement Deref and DerefMut to enable the direct use of the lexer's methods
impl<'a> Deref for C1Parser<'a> {
    type Target = C1Lexer<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for C1Parser<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> C1Parser<'a> {
    pub fn parse(text: &str) -> ParseResult {
        let mut parser = Self::initialize_parser(text);
        parser.program()
    }

    fn initialize_parser(text: &str) -> C1Parser {
        C1Parser(C1Lexer::new(text))
    }

    /// program ::= ( functiondefinition )* <EOF>
    pub fn program(&mut self) -> ParseResult {
        while let Some(_) = self.current_token() {
            self.function_definition()?;
        }

        Ok(())
    }

    fn function_definition(&mut self) -> ParseResult {
        self.p_type()
            .and_then(|_| self.check_and_eat_token(&C1Token::Identifier, "Expected function name"))
            .and_then(|_| self.check_and_eat_token(&C1Token::LeftParenthesis, r#"Expected "(""#))
            .and_then(|_| self.check_and_eat_token(&C1Token::RightParenthesis, r#"Expected ")""#))
            .and_then(|_| self.check_and_eat_token(&C1Token::LeftBrace, r#"Expected "{""#))
            .and_then(|_| self.statementlist())
            .and_then(|_| self.check_and_eat_token(&C1Token::RightBrace, r#"Expected "}""#))
            .map_err(|err| err + ", in function definition")
    }

    fn functioncall(&mut self) -> ParseResult {
        self.check_and_eat_token(&C1Token::Identifier, "Expected <ID>")
            .and_then(|_| self.check_and_eat_token(&C1Token::LeftParenthesis, r#"Expected "(""#))
            .and_then(|_| self.check_and_eat_token(&C1Token::RightParenthesis, r#"Expected ")""#))
            .map_err(|err| err + ", in functioncall")
    }

    fn statementlist(&mut self) -> ParseResult {
        let mut m = self.mark();

        while let Ok(_) = self.block() {
            self.pop_mark();
            m = self.mark();
        }

        self.undo(m);
        self.pop_mark();

        Ok(())
    }

    fn block(&mut self) -> ParseResult {
        let m = self.mark();
        let res = self
            .check_and_eat_token(&C1Token::LeftBrace, r#"Expected "{""#)
            .and_then(|_| self.statementlist())
            .and_then(|_| self.check_and_eat_token(&C1Token::RightBrace, r#"Expected "}""#))
            .or_else(|_| {
                self.undo(m);
                self.statement()
            });
        self.pop_mark();
        res
    }

    fn statement(&mut self) -> ParseResult {
        let m = self.mark();
        let res = self
            .ifstatement()
            .or_else(|_| {
                self.undo(m);
                self.returnstatement()?;
                self.check_and_eat_token(
                    &C1Token::Semicolon,
                    "Expected semicolon after return statement",
                )
            })
            .or_else(|_| {
                self.undo(m);
                self.printf()?;
                self.check_and_eat_token(&C1Token::Semicolon, "Expected semicolon after printf")
            })
            .or_else(|_| {
                self.undo(m);
                self.statassignment()?;
                self.check_and_eat_token(
                    &C1Token::Semicolon,
                    "Expected semicolon after stat assignment",
                )
            })
            .or_else(|_| {
                self.undo(m);
                self.functioncall()?;
                self.check_and_eat_token(
                    &C1Token::Semicolon,
                    "Expected semicolon after function call",
                )
            });

        self.pop_mark();

        res
    }

    fn ifstatement(&mut self) -> ParseResult {
        // <KW_IF> "(" assignment ")" block
        self.check_and_eat_token(&C1Token::KwIf, r#"Expected "if""#)?;
        self.check_and_eat_token(&C1Token::LeftParenthesis, r#"Expected "(""#)?;
        self.assignment()?;
        self.check_and_eat_token(&C1Token::RightParenthesis, r#"Expected ")""#)?;
        self.block()
    }

    fn returnstatement(&mut self) -> ParseResult {
        self.check_and_eat_token(&C1Token::KwReturn, r#"Expected "return""#)?;
        let _ = self.assignment(); // optional
        Ok(())
    }

    fn printf(&mut self) -> ParseResult {
        self.check_and_eat_token(&C1Token::KwPrintf, r#"Expected "printf""#)?;
        self.check_and_eat_token(&C1Token::LeftParenthesis, r#"Expected "(""#)?;
        self.assignment()?;
        self.check_and_eat_token(&C1Token::RightParenthesis, r#"Expected ")""#)
    }

    fn p_type(&mut self) -> ParseResult {
        self.any_match_and_eat(
            &[
                C1Token::KwBoolean,
                C1Token::KwFloat,
                C1Token::KwInt,
                C1Token::KwVoid,
            ],
            "Expected type",
        )
    }

    fn statassignment(&mut self) -> ParseResult {
        self.check_and_eat_token(&C1Token::Identifier, r#"Expected <ID>"#)
            .and_then(|_| self.check_and_eat_token(&C1Token::Assign, r#"Expected "=""#))
            .and_then(|_| self.assignment())
    }

    fn assignment(&mut self) -> ParseResult {
        let m = self.mark();

        let res = self
            .check_and_eat_token(&C1Token::Identifier, r#"Expected "<ID>""#)
            .and_then(|_| self.check_and_eat_token(&C1Token::Assign, r#"Expected "=""#))
            .and_then(|_| self.assignment())
            .or_else(|_| {
                self.undo(m);
                self.expr()
            });

        self.pop_mark();

        res
    }

    fn expr(&mut self) -> ParseResult {
        self.simpexpr()?;
        let m = self.mark();

        let ops = [
            C1Token::Equal,
            C1Token::NotEqual,
            C1Token::LessEqual,
            C1Token::GreaterEqual,
            C1Token::Less,
            C1Token::Greater,
        ];

        let _ = self
            .any_match_and_eat(&ops, "")
            .and_then(|_| self.simpexpr())
            .or_else(|_| -> ParseResult {
                self.undo(m);
                Ok(())
            }); // optional

        self.pop_mark();

        Ok(())
    }

    fn simpexpr(&mut self) -> ParseResult {
        let _ = self.check_and_eat_token(&C1Token::Minus, ""); // optional
        self.term().map_err(|err| err + ", in simpexpr")?;
        let mut m = self.mark();

        while let Ok(_) = self
            .any_match_and_eat(&[C1Token::Plus, C1Token::Minus, C1Token::Or], "")
            .and_then(|_| self.term())
        {
            self.pop_mark();
            m = self.mark();
        }

        self.undo(m);
        self.pop_mark();

        Ok(())
    }

    fn term(&mut self) -> ParseResult {
        self.factor().map_err(|err| err + ", in term")?;
        let mut m = self.mark();

        while let Ok(_) = self
            .any_match_and_eat(&[C1Token::Asterisk, C1Token::Slash, C1Token::And], "")
            .and_then(|_| self.factor())
        {
            self.pop_mark();
            m = self.mark();
        }

        self.undo(m);
        self.pop_mark();

        Ok(())
    }

    fn factor(&mut self) -> ParseResult {
        let m = self.mark();

        let res = self
            .check_and_eat_token(&C1Token::ConstInt, "")
            .or_else(|_| {
                self.undo(m);
                self.check_and_eat_token(&C1Token::ConstFloat, "")
            })
            .or_else(|_| {
                self.undo(m);
                self.check_and_eat_token(&C1Token::ConstBoolean, "")
            })
            .or_else(|_| {
                self.undo(m);
                self.functioncall()
            })
            .or_else(|_| {
                self.undo(m);
                self.check_and_eat_token(&C1Token::Identifier, "")
            })
            .or_else(|_| {
                self.undo(m);
                self.check_and_eat_token(&C1Token::LeftParenthesis, "Expected <FACTOR>")?;
                self.assignment()?;
                self.check_and_eat_token(&C1Token::RightParenthesis, "Expected ')'")
            })
            .or_else(|err| {
                self.undo(m);
                Err(err)
            });

        self.pop_mark();

        res
    }

    /// Check whether the current token is equal to the given token. If yes, consume it, otherwise
    /// return an error with the given error message
    fn check_and_eat_token(&mut self, token: &C1Token, reason: &str) -> ParseResult {
        if self.current_matches(token) {
            self.advance();
            Ok(())
        } else {
            let err = match self.current_token() {
                None => format!("{}. Reached EOF", reason),
                Some(_) => format!(
                    "Unexpected token: {} \n at line {:?} while trying to parse: '{}'",
                    reason,
                    self.current_line_number().unwrap(),
                    self.current_text().unwrap()
                ),
            };
            Err(err)
        }
    }

    /// Check whether the given token matches the current token
    fn current_matches(&self, token: &C1Token) -> bool {
        match &self.current_token() {
            None => false,
            Some(current) => current == token,
        }
    }

    /// Check whether any of the tokens matches the current token, then consume it
    fn any_match_and_eat(&mut self, token: &[C1Token], reason: &str) -> ParseResult {
        if token
            .iter()
            .any(|t| self.check_and_eat_token(t, "").is_ok())
        {
            Ok(())
        } else {
            let err = match self.current_token() {
                None => format!("{}. Reached EOF", reason),
                Some(_) => format!(
                    "Unexpected token: {} \n at line {:?} while trying to parse: '{}'",
                    reason,
                    self.current_line_number().unwrap(),
                    self.current_text().unwrap()
                ),
            };
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::{C1Parser, ParseResult};

    fn call_method<'a, F>(parse_method: F, text: &'static str) -> ParseResult
    where
        F: Fn(&mut C1Parser<'a>) -> ParseResult,
    {
        let mut parser = C1Parser::initialize_parser(text);
        if let Err(message) = parse_method(&mut parser) {
            eprintln!("Parse Error: {}", message);
            Err(message)
        } else {
            println!("{:?}", parser.current_token());
            Ok(())
        }
    }

    #[test]
    fn parse_empty_program() {
        let result = C1Parser::parse("");
        assert_eq!(result, Ok(()));

        let result = C1Parser::parse("   ");
        assert_eq!(result, Ok(()));

        let result = C1Parser::parse("// This is a valid comment!");
        assert_eq!(result, Ok(()));

        let result = C1Parser::parse("/* This is a valid comment!\nIn two lines!*/\n");
        assert_eq!(result, Ok(()));

        let result = C1Parser::parse("  \n ");
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn fail_invalid_program() {
        let result = C1Parser::parse("  bool  ");
        println!("{:?}", result);
        assert!(result.is_err());

        let result = C1Parser::parse("int x = 0;");
        println!("{:?}", result);
        assert!(result.is_err());

        let result = C1Parser::parse("// A valid comment\nInvalid line.");
        println!("{:?}", result);
        assert!(result.is_err());
    }

    #[test]
    fn valid_function() {
        // let result = C1Parser::parse("  void foo() {}  ");
        // assert!(result.is_ok(), "Parse result: {}", result.err().unwrap());

        let result = C1Parser::parse("int bar() {return 0;}");
        assert!(result.is_ok(), "Parse result: {}", result.err().unwrap());

        let result = C1Parser::parse(
            "float calc() {\n\
        x = 1.0;
        y = 2.2;
        return x + y;
        \n\
        }",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn fail_invalid_function() {
        let result = C1Parser::parse("  void foo()) {}  ");
        println!("{:?}", result);
        assert!(result.is_err());

        let result = C1Parser::parse("const bar() {return 0;}");
        println!("{:?}", result);
        assert!(result.is_err());

        let result = C1Parser::parse(
            "int bar() {
                                                          return 0;
                                                     int foo() {}",
        );
        println!("{:?}", result);
        assert!(result.is_err());

        let result = C1Parser::parse(
            "float calc(int invalid) {\n\
        int x = 1.0;
        int y = 2.2;
        return x + y;
        \n\
        }",
        );
        println!("{:?}", result);
        assert!(result.is_err());
    }

    #[test]
    fn valid_functioncall() {
        assert!(call_method(C1Parser::functioncall, "foo()").is_ok());
        assert!(call_method(C1Parser::functioncall, "foo( )").is_ok());
        assert!(call_method(C1Parser::functioncall, "bar23( )").is_ok());
    }

    #[test]
    fn fail_invalid_functioncall() {
        assert!(call_method(C1Parser::functioncall, "foo)").is_err());
        assert!(call_method(C1Parser::functioncall, "foo{ )").is_err());
        assert!(call_method(C1Parser::functioncall, "bar _foo( )").is_err());
    }

    #[test]
    fn valid_statementlist() {
        assert!(call_method(C1Parser::statementlist, "return 3;").is_ok());
        assert!(call_method(
            C1Parser::statementlist,
            "x = 4;\ny = 2.1;"
        )
        .is_ok());
        assert!(call_method(
            C1Parser::statementlist,
            "x = 4;\n\
        {\
        foo();\n\
        }"
        )
        .is_ok());
        assert!(call_method(C1Parser::statementlist, "{x = 4;}\ny = 1;\nfoo();\n{}").is_ok());
    }

    #[test]
    fn valid_ifstatement() {
        assert!(call_method(C1Parser::ifstatement, "if(x == y) {}").is_ok());
        assert!(call_method(C1Parser::ifstatement, "if(z) {}").is_ok());
        assert!(call_method(C1Parser::ifstatement, "if(true) {}").is_ok());
        assert!(call_method(C1Parser::ifstatement, "if(false) {}").is_ok());
    }

    #[test]
    fn fail_invalid_ifstatement() {
        assert!(call_method(C1Parser::ifstatement, "if(x == ) {}").is_err());
        assert!(call_method(C1Parser::ifstatement, "if( == y) {}").is_err());
        assert!(call_method(C1Parser::ifstatement, "if(> z) {}").is_err());
        assert!(call_method(C1Parser::ifstatement, "if( {}").is_err());
        assert!(call_method(C1Parser::ifstatement, "if(false) }").is_err());
    }

    #[test]
    fn valid_returnstatement() {
        assert!(call_method(C1Parser::returnstatement, "return x").is_ok());
        assert!(call_method(C1Parser::returnstatement, "return 1").is_ok());
        assert!(call_method(C1Parser::returnstatement, "return").is_ok());
    }

    #[test]
    fn fail_invalid_returnstatement() {
        assert!(call_method(C1Parser::returnstatement, "1").is_err());
    }

    #[test]
    fn valid_printf_statement() {
        assert!(call_method(C1Parser::printf, " printf(a+b)").is_ok());
        assert!(call_method(C1Parser::printf, "printf( 1)").is_ok());
        assert!(call_method(C1Parser::printf, "printf(a - c)").is_ok());
    }

    #[test]
    fn fail_invalid_printf_statement() {
        assert!(call_method(C1Parser::printf, "printf( ").is_err());
        assert!(call_method(C1Parser::printf, "printf(printf)").is_err());
        assert!(call_method(C1Parser::printf, "Printf()").is_err());
    }

    #[test]
    fn valid_p_type() {
        assert!(call_method(C1Parser::p_type, "void").is_ok());
        assert!(call_method(C1Parser::p_type, "bool").is_ok());
        assert!(call_method(C1Parser::p_type, "int").is_ok());
        assert!(call_method(C1Parser::p_type, "float").is_ok());
    }

    #[test]
    fn valid_assignment() {
        assert!(call_method(C1Parser::assignment, "b > blub()").is_ok());
        assert!(call_method(C1Parser::assignment, "x = y + 1").is_ok());
        assert!(call_method(C1Parser::assignment, "1 + 2").is_ok());
    }

    #[test]
    fn valid_statassignment() {
        assert!(call_method(C1Parser::statassignment, "x = y").is_ok());
        assert!(call_method(C1Parser::statassignment, "x = y").is_ok());
        assert!(call_method(C1Parser::statassignment, "x = y + t").is_ok());
    }

    #[test]
    fn valid_factor() {
        assert!(call_method(C1Parser::factor, "4").is_ok());
        assert!(call_method(C1Parser::factor, "1.2").is_ok());
        assert!(call_method(C1Parser::factor, "true").is_ok());
        assert!(call_method(C1Parser::factor, "foo()").is_ok());
        assert!(call_method(C1Parser::factor, "x").is_ok());
        assert!(call_method(C1Parser::factor, "(x + y)").is_ok());
    }

    #[test]
    fn fail_invalid_factor() {
        assert!(call_method(C1Parser::factor, "if").is_err());
        assert!(call_method(C1Parser::factor, "(x +").is_err());
        assert!(call_method(C1Parser::factor, "bool").is_err());
    }

    #[test]
    fn multiple_functions() {
        assert!(call_method(
            C1Parser::program,
            "void main() { hello();}\nfloat bar() {return 1.0;}"
        )
        .is_ok());
    }
}
