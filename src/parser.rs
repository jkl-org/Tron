use crate::expr::{Expr, Expr::*, LiteralValue};
use crate::scanner::{Token, TokenType, TokenType::*};
use crate::stmt::Stmt;
use colored::Colorize;
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    next_id: usize,
}
#[derive(Debug)]
enum FunctionKind {
    Function,
}
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            next_id: 0,
        }
    }
    fn get_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = vec![];
        let mut errs = vec![];
        while !self.is_at_end() {
            let stmt = self.declaration();
            match stmt {
                Ok(s) => stmts.push(s),
                Err(msg) => {
                    errs.push(msg.red().to_string());
                    self.synchronize();
                }
            }
        }
        if errs.len() == 0 {
            Ok(stmts)
        } else {
            Err(errs.join("\n"))
        }
    }
    fn declaration(&mut self) -> Result<Stmt, String> {
        if self.match_token(Var) {
            self.var_declaration()
        } else if self.match_token(Fun) {
            self.function(FunctionKind::Function)
        } else {
            self.statement()
        }
    }
    fn function(&mut self, kind: FunctionKind) -> Result<Stmt, String> {
        let name = self.consume(Identifier, &format!("Expected {kind:?} name"))?;
        if self.match_token(Gets) {
            let cmd_body = self.consume(StringLit, "Expected command body")?;
            self.consume(Semicolon, "Expected ';' after command body")?;
            return Ok(Stmt::CmdFunction {
                name,
                cmd: cmd_body.lexeme,
            });
        }
        self.consume(LeftParen, &format!("Expected '(' after {kind:?} name"))?;
        let mut parameters = vec![];
        
        if !self.check(RightParen) {
            loop {
                if parameters.len() >= 255 {
                    let location = self.peek().line_number;
                    return Err(format!(
                        "Error 111: Line {location}: Cant have more than 255 arguments"
                    )
                    .red()
                    .to_string());
                }
                let param = self.consume(Identifier, "Expected parameter name")?;
                parameters.push(param);
                if !self.match_token(Comma) {
                    break;
                }
            }
        }
        self.consume(RightParen, "Expected ')' after parameters.")?;
        self.consume(LeftBrace, &format!("Expected '{{' before {kind:?} body."))?;
        let body = match self.block_statement()? {
            Stmt::Block { statements } => statements,
            _ => panic!("Block statement parsed something that was not a block"),
        };
        Ok(Stmt::Function {
            name,
            params: parameters,
            body,
        })
    }


    fn var_declaration(&mut self) -> Result<Stmt, String> {
        let token = self.consume(Identifier, "Expected variable name")?;
        let initializer;
        if self.match_token(Equal) {
            initializer = self.expression()?;
        } else {
            initializer = Literal {
                id: self.get_id(),
                value: LiteralValue::Nil,
            };
        }
        self.consume(Semicolon, "Expected ';' after variable declaration")?;
        Ok(Stmt::Var {
            name: token,
            initializer,
        })
    }
    fn statement(&mut self) -> Result<Stmt, String> {
        if self.match_token(Print) {
            self.print_statement()
        } else if self.match_token(Input) {
            self.inputs_statement()
        } else if self.match_token(Errors) {
            self.error_statement()
        } else if self.match_token(Exits) {
            self.exits_statement()
        } else if self.match_token(Import) {
            self.import_statement()
        } else if self.match_token(LeftBrace) {
            self.block_statement()
        } else if self.match_token(If) {
            self.if_statement()
        } else if self.match_token(While) {
            self.while_statement()
        } else if self.match_token(Bench) {
            self.bench_statement()
        } else if self.match_token(For) {
            self.for_statement()
        } else if self.match_token(Return) {
            self.return_statement()
        } else {
            self.expression_statement()
        }
    }
    fn return_statement(&mut self) -> Result<Stmt, String> {
        let keyword = self.previous();
        let value;
        if !self.check(Semicolon) {
            value = Some(self.expression()?);
        } else {
            value = None;
        }
        self.consume(Semicolon, "Expected ';' after return value;")?;
        Ok(Stmt::ReturnStmt { keyword, value })
    }
    fn for_statement(&mut self) -> Result<Stmt, String> {
        let initializer;
        if self.match_token(Semicolon) {
            initializer = None;
        } else if self.match_token(Var) {
            let var_decl = self.var_declaration()?;
            initializer = Some(var_decl);
        } else {
            let expr = self.expression_statement()?;
            initializer = Some(expr);
        }
        let condition;
        if !self.check(Semicolon) {
            let expr = self.expression()?;
            condition = Some(expr);
        } else {
            condition = None;
        }
        self.consume(Semicolon, "Expected ';' after loop condition.")?;
        let increment;
        if !self.check(RightParen) {
            let expr = self.expression()?;
            increment = Some(expr);
        } else {
            increment = None;
        }
        let mut body = self.statement()?;
        if let Some(incr) = increment {
            body = Stmt::Block {
                statements: vec![
                    Box::new(body),
                    Box::new(Stmt::Expression { expression: incr }),
                ],
            };
        }
        let cond;
        match condition {
            None => {
                cond = Expr::Literal {
                    id: self.get_id(),
                    value: LiteralValue::True,
                }
            }
            Some(c) => cond = c,
        }
        body = Stmt::WhileStmt {
            condition: cond,
            body: Box::new(body),
        };
        if let Some(init) = initializer {
            body = Stmt::Block {
                statements: vec![Box::new(init), Box::new(body)],
            };
        }
        Ok(body)
    }
    fn while_statement(&mut self) -> Result<Stmt, String> {
        let condition = self.expression()?;
        let body = self.statement()?;
        Ok(Stmt::WhileStmt {
            condition,
            body: Box::new(body),
        })
    }

    fn bench_statement(&mut self) -> Result<Stmt, String> {
        let body = self.statement()?;
        Ok(Stmt::BenchStmt {
            body: Box::new(body),
        })
    }
    
    fn if_statement(&mut self) -> Result<Stmt, String> {
        let predicate = self.expression()?;
        let then = Box::new(self.statement()?);
        let els = if self.match_token(Else) {
            let stm = self.statement()?;
            Some(Box::new(stm))
        } else {
            None
        };
        Ok(Stmt::IfStmt {
            predicate,
            then,
            els,
        })
    }
    fn block_statement(&mut self) -> Result<Stmt, String> {
        let mut statements = vec![];
        while !self.check(RightBrace) && !self.is_at_end() {
            let decl = self.declaration()?;
            statements.push(Box::new(decl));
        }
        self.consume(RightBrace, "Expected '}' after a block")?;
        Ok(Stmt::Block { statements })
    }
    fn print_statement(&mut self) -> Result<Stmt, String> {
        let value = self.expression()?;
        self.consume(Semicolon, "Expected ';' after value.")?;
        Ok(Stmt::Print { expression: value })
    }
    fn inputs_statement(&mut self) -> Result<Stmt, String> {
        let value = self.expression()?;
        self.consume(Semicolon, "Expected ';' after value.")?;
        Ok(Stmt::Input { expression: value })
    }
    fn error_statement(&mut self) -> Result<Stmt, String> {
        let value = self.expression()?;
        self.consume(Semicolon, "Expected ';' after value.")?;
        Ok(Stmt::Errors { expression: value })
    }
    fn exits_statement(&mut self) -> Result<Stmt, String> {
        self.consume(Semicolon, "Expected ';' after value.")?;
        Ok(Stmt::Exits {})
    }
    fn import_statement(&mut self) -> Result<Stmt, String> {
        let value = self.expression()?;
        self.consume(Semicolon, "Expected ';' after value.")?;
        Ok(Stmt::Import { expression: value })
    }
    fn expression_statement(&mut self) -> Result<Stmt, String> {
        let expr = self.expression()?;
        self.consume(Semicolon, "Expected ';' after expression.")?;
        Ok(Stmt::Expression { expression: expr })
    }
    fn expression(&mut self) -> Result<Expr, String> {
        self.assignment()
    }
    fn function_expression(&mut self) -> Result<Expr, String> {
        let paren = self.consume(LeftParen, "Expected '(' after anonymous function")?;
        let mut parameters = vec![];
        if !self.check(RightParen) {
            loop {
                if parameters.len() >= 255 {
                    let location = self.peek().line_number;
                    return Err(format!(
                        "Error 111: Line {location}: Cant have more than 255 arguments"
                    )
                    .red()
                    .to_string());
                }
                let param = self.consume(Identifier, "Expected parameter name")?;
                parameters.push(param);
                if !self.match_token(Comma) {
                    break;
                }
            }
        }
        self.consume(
            RightParen,
            "Expected ')' after anonymous function parameters",
        )?;
        self.consume(
            LeftBrace,
            "Expected '{' after anonymous function declaration",
        )?;
        let body = match self.block_statement()? {
            Stmt::Block { statements } => statements,
            _ => panic!("Block statement parsed something that was not a block"),
        };
        Ok(Expr::AnonFunction {
            id: self.get_id(),
            paren,
            arguments: parameters,
            body,
        })
    }
    fn assignment(&mut self) -> Result<Expr, String> {
        let expr = self.pipe()?;
        if self.match_token(Equal) {
            let value = self.expression()?;
            match expr {
                Variable { id: _, name } => Ok(Assign {
                    id: self.get_id(),
                    name,
                    value: Box::from(value),
                }),
                Get {
                    id: _,
                    object,
                    name,
                } => Ok(Set {
                    id: self.get_id(),
                    object,
                    name,
                    value: Box::new(value),
                }),
                _ => Err("Error 112: Invalid assignment target."
                    .to_string()
                    .red()
                    .to_string()),
            }
        } else {
            Ok(expr)
        }
    }
    fn pipe(&mut self) -> Result<Expr, String> {
        let mut expr = self.or()?;
        while self.match_token(Pipe) {
            let pipe = self.previous();
            let function = self.or()?;
            expr = Call {
                id: self.get_id(),
                callee: Box::new(function),
                paren: pipe,
                arguments: vec![expr],
            };
        }
        Ok(expr)
    }
    fn or(&mut self) -> Result<Expr, String> {
        let mut expr = self.and()?;
        while self.match_token(Or) {
            let operator = self.previous();
            let right = self.and()?;
            expr = Logical {
                id: self.get_id(),
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut expr = self.equality()?;
        while self.match_token(And) {
            let operator = self.previous();
            let right = self.equality()?;
            expr = Logical {
                id: self.get_id(),
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }
    fn equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.comparison()?;
        while self.match_tokens(&[BangEqual, EqualEqual]) {
            let operator = self.previous();
            let rhs = self.comparison()?;
            expr = Binary {
                id: self.get_id(),
                left: Box::from(expr),
                operator,
                right: Box::from(rhs),
            };
        }
        Ok(expr)
    }
    fn comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.term()?;
        while self.match_tokens(&[Greater, GreaterEqual, Less, LessEqual]) {
            let op = self.previous();
            let rhs = self.term()?;
            expr = Binary {
                id: self.get_id(),
                left: Box::from(expr),
                operator: op,
                right: Box::from(rhs),
            };
        }
        Ok(expr)
    }
    fn term(&mut self) -> Result<Expr, String> {
        let mut expr = self.factor()?;
        while self.match_tokens(&[Minus, Plus, PlusEqual, MinusEqual, Random]) {
            let op = self.previous();
            let rhs = self.factor()?;
            expr = Binary {
                id: self.get_id(),
                left: Box::from(expr),
                operator: op,
                right: Box::from(rhs),
            };
        }
        Ok(expr)
    }
    fn factor(&mut self) -> Result<Expr, String> {
        let mut expr = self.unary()?;
        while self.match_tokens(&[Slash, Star, Power, Cube, Root, CubicRoot]) {
            let op = self.previous();
            let rhs = self.unary()?;
            expr = Binary {
                id: self.get_id(),
                left: Box::from(expr),
                operator: op,
                right: Box::from(rhs),
            };
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.match_tokens(&[
            Bang, Minus, Increment, Decrement, Sin, Cos, Tan, In, Round, Floor, Percent, ToDeg,
            ToRad, ASin, ACos, ATan, Parse, Num,
        ]) {
            let op = self.previous();
            let rhs = self.unary()?;
            Ok(Unary {
                id: self.get_id(),
                operator: op,
                right: Box::from(rhs),
            })
        } else {
            self.call()
        }
    }
    #[allow(dead_code)]
    fn sunary(&mut self) -> Result<Expr, String> {
        if self.match_tokens(&[Sin]) {
            let rhs = self.sunary()?;
            let op = self.after();
            Ok(SUnary {
                id: self.get_id(),
                operator: op,
                left: Box::from(rhs),
            })
        } else {
            self.call()
        }
    }
    fn call(&mut self) -> Result<Expr, String> {
        let mut expr = self.primary()?;
        loop {
            if self.match_token(LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(Dot) {
                let name = self.consume(Identifier, "Expected token after dot-accessor")?;
                expr = Get {
                    id: self.get_id(),
                    object: Box::new(expr),
                    name,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }
    fn finish_call(&mut self, callee: Expr) -> Result<Expr, String> {
        let mut arguments = vec![];
        if !self.check(RightParen) {
            loop {
                let arg = self.expression()?;
                arguments.push(arg);
                if arguments.len() >= 255 {
                    let location = self.peek().line_number;
                    return Err(format!(
                        "Error 111: Line {location}: Cant have more than 255 arguments"
                    )
                    .red()
                    .to_string());
                } else if !self.match_token(Comma) {
                    break;
                }
            }
        }
        let paren = self.consume(RightParen, "Expected ')' after arguments.")?;
        Ok(Call {
            id: self.get_id(),
            callee: Box::new(callee),
            paren,
            arguments,
        })
    }
    
    fn primary(&mut self) -> Result<Expr, String> {
        let token = self.peek();
        let result;
        match token.token_type {
            LeftParen => {
                self.advance();
                let expr = self.expression()?;
                self.consume(RightParen, "Expected ')'")?;
                result = Grouping {
                    id: self.get_id(),
                    expression: Box::from(expr),
                };
            }
            False | True | Nil | Number | StringLit => {
                self.advance();
                result = Literal {
                    id: self.get_id(),
                    value: LiteralValue::from_token(token),
                }
            }
            Identifier => {
                self.advance();
                result = Variable {
                    id: self.get_id(),
                    name: self.previous(),
                };
            }
            Fun => {
                self.advance();
                result = self.function_expression()?;
            }
            _ => {
                return Err("Error 113: Expected expression"
                    .to_string()
                    .red()
                    .to_string())
            }
        }
        Ok(result)
    }
    fn consume(&mut self, token_type: TokenType, msg: &str) -> Result<Token, String> {
        let token = self.peek();
        if token.token_type == token_type {
            self.advance();
            let token = self.previous();
            Ok(token)
        } else {
            Err(format!("Line {}: {}", token.line_number, msg)
                .red()
                .to_string())
        }
    }
    fn check(&mut self, typ: TokenType) -> bool {
        self.peek().token_type == typ
    }
    fn match_token(&mut self, typ: TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            if self.peek().token_type == typ {
                self.advance();
                true
            } else {
                false
            }
        }
    }
    fn match_tokens(&mut self, typs: &[TokenType]) -> bool {
        for typ in typs {
            if self.match_token(*typ) {
                return true;
            }
        }
        false
    }
    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }
    fn peek(&mut self) -> Token {
        self.tokens[self.current].clone()
    }
    fn previous(&mut self) -> Token {
        self.tokens[self.current - 1].clone()
    }
    fn after(&mut self) -> Token {
        self.tokens[self.current + 1].clone()
    }
    fn is_at_end(&mut self) -> bool {
        self.peek().token_type == Eof
    }
    fn synchronize(&mut self) {
        self.advance();
        while !self.is_at_end() {
            if self.previous().token_type == Semicolon {
                return;
            }
            match self.peek().token_type {
                Fun | Var | For | If | Input | Errors | While | Bench | Print | Return | Import
                | Exits => return,
                _ => (),
            }
            self.advance();
        }
    }
}
