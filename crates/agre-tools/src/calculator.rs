use crate::{Tool, ToolError};
use agre_core::{ParameterSchema, ToolSchema};
use async_trait::async_trait;
use serde_json::Value;

pub struct Calculator;

#[async_trait]
impl Tool for Calculator {
  fn name(&self) -> &str {
    "calculator"
  }

  fn description(&self) -> &str {
    "Evaluate a basic arithmetic expression."
  }

  fn schema(&self) -> ToolSchema {
    ToolSchema::object(
      vec![(
        "expression",
        ParameterSchema::string(
          "Arithmetic expression using numbers, +, -, *, /, % and parantheses ().",
        ),
      )],
      &["expression"],
    )
  }

  async fn call(&self, args: Value) -> Result<Value, ToolError> {
    let expression = args
      .get("expression")
      .and_then(Value::as_str)
      .ok_or_else(|| ToolError::InvalidArguments("expected string field 'expression'".into()))?;

    let result = evaluate(expression).map_err(ToolError::Execution)?;

    if !result.is_finite() {
      return Err(ToolError::Execution(
        "calculation produced a non-finite result".into(),
      ));
    }

    Ok(serde_json::json!({
      "result": result
    }))
  }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
  Number(f64),
  Plus,
  Minus,
  Star,
  Slash,
  Percent,
  LParen,
  RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
  let chars: Vec<char> = input.chars().collect();
  let mut tokens = Vec::<Token>::new();
  let mut i = 0;

  while i < chars.len() {
    let c = chars[i];

    match c {
      ' ' | '\t' | '\n' | '\r' => {
        i += 1;
      }

      '+' => {
        tokens.push(Token::Plus);
        i += 1;
      }

      '-' => {
        tokens.push(Token::Minus);
        i += 1;
      }

      '*' => {
        tokens.push(Token::Star);
        i += 1;
      }

      '/' => {
        tokens.push(Token::Slash);
        i += 1;
      }

      '%' => {
        tokens.push(Token::Percent);
        i += 1;
      }

      '(' => {
        tokens.push(Token::LParen);
        i += 1;
      }

      ')' => {
        tokens.push(Token::RParen);
        i += 1;
      }

      '0'..='9' | '.' => {
        let start = i;
        let mut dots = 0;

        while i < chars.len() {
          match chars[i] {
            '0'..='9' => i += 1,

            '.' => {
              dots += 1;

              if dots > 1 {
                return Err("Invalid number".into());
              }

              i += 1;
            }

            _ => break,
          }
        }

        let text: String = chars[start..i].iter().collect();

        let number = text
          .parse::<f64>()
          .map_err(|_| format!("Invalid number: {}", text))?;

        tokens.push(Token::Number(number));
      }

      _ => {
        return Err(format!("Unexpected character: {}", c));
      }
    }
  }

  Ok(tokens)
}

#[derive(Debug)]
enum Expr {
  Number(f64),
  Add(Box<Expr>, Box<Expr>),
  Sub(Box<Expr>, Box<Expr>),
  Mul(Box<Expr>, Box<Expr>),
  Div(Box<Expr>, Box<Expr>),
  Mod(Box<Expr>, Box<Expr>),
  Neg(Box<Expr>),
  Pos(Box<Expr>),
}

impl Expr {
  fn evaluate(&self) -> Result<f64, String> {
    match self {
      Expr::Number(n) => Ok(*n),

      Expr::Add(a, b) => Ok(a.evaluate()? + b.evaluate()?),

      Expr::Sub(a, b) => Ok(a.evaluate()? - b.evaluate()?),

      Expr::Mul(a, b) => Ok(a.evaluate()? * b.evaluate()?),

      Expr::Div(a, b) => {
        let divisor = b.evaluate()?;

        if divisor == 0.0 {
          return Err("division by zero".into());
        }

        Ok(a.evaluate()? / divisor)
      }

      Expr::Mod(a, b) => {
        let divisor = b.evaluate()?;

        if divisor == 0.0 {
          return Err("modulo by zero".into());
        }

        Ok(a.evaluate()? % divisor)
      }

      Expr::Neg(expr) => Ok(-expr.evaluate()?),

      Expr::Pos(expr) => Ok(expr.evaluate()?),
    }
  }
}

struct Parser {
  tokens: Vec<Token>,
  pos: usize,
}

impl Parser {
  fn new(tokens: Vec<Token>) -> Self {
    Self { tokens, pos: 0 }
  }

  fn peek(&self) -> Option<&Token> {
    self.tokens.get(self.pos)
  }

  fn advance(&mut self) -> Option<Token> {
    let token = self.tokens.get(self.pos).cloned();

    if token.is_some() {
      self.pos += 1;
    }

    token
  }

  fn parse_expression(&mut self) -> Result<Expr, String> {
    let mut left = self.parse_term()?;

    loop {
      match self.peek() {
        Some(Token::Plus) => {
          self.advance();

          let right = self.parse_term()?;

          left = Expr::Add(Box::new(left), Box::new(right));
        }

        Some(Token::Minus) => {
          self.advance();

          let right = self.parse_term()?;

          left = Expr::Sub(Box::new(left), Box::new(right));
        }

        _ => break,
      }
    }

    Ok(left)
  }

  fn parse_term(&mut self) -> Result<Expr, String> {
    let mut left = self.parse_unary()?;

    loop {
      match self.peek() {
        Some(Token::Star) => {
          self.advance();

          let right = self.parse_unary()?;

          left = Expr::Mul(Box::new(left), Box::new(right));
        }

        Some(Token::Slash) => {
          self.advance();

          let right = self.parse_unary()?;

          left = Expr::Div(Box::new(left), Box::new(right));
        }

        Some(Token::Percent) => {
          self.advance();

          let right = self.parse_unary()?;

          left = Expr::Mod(Box::new(left), Box::new(right));
        }

        _ => break,
      }
    }

    Ok(left)
  }

  fn parse_unary(&mut self) -> Result<Expr, String> {
    match self.peek() {
      Some(Token::Minus) => {
        self.advance();

        let expr = self.parse_unary()?;

        Ok(Expr::Neg(Box::new(expr)))
      }

      Some(Token::Plus) => {
        self.advance();

        let expr = self.parse_unary()?;

        Ok(Expr::Pos(Box::new(expr)))
      }

      _ => self.parse_primary(),
    }
  }

  fn parse_primary(&mut self) -> Result<Expr, String> {
    match self.advance() {
      Some(Token::Number(n)) => Ok(Expr::Number(n)),

      Some(Token::LParen) => {
        let expr = self.parse_expression()?;

        match self.advance() {
          Some(Token::RParen) => Ok(expr),
          _ => Err("expected ')'".into()),
        }
      }

      Some(token) => Err(format!("unexpected token: {:?}", token)),

      None => Err("unexpected end of expression".into()),
    }
  }
}

fn evaluate(input: &str) -> Result<f64, String> {
  let tokens = tokenize(input)?;
  let mut parser = Parser::new(tokens);

  let expr = parser.parse_expression()?;

  if parser.peek().is_some() {
    return Err("Unexpected tokens after expression".into());
  }

  expr.evaluate()
}
