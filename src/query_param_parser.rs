use mongodb::bson::{Bson, Document, bson};

type Number = f64;

#[derive(Debug, Clone)]
pub enum Operand {
    Str(String),
    Num(Number),
}

impl PartialEq for Operand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Operand::Str(a), Operand::Str(b)) => a == b,
            (Operand::Num(a), Operand::Num(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for Operand {}

#[derive(Debug, Clone)]
pub enum LexItem {
    Operator(String),  // 'eq', 'ne', 'lt', 'gt', 'lte', 'gte'
    SpecialChar(char), // Specoal characters like `(` `)` `,` `.`
    Operand(Operand),
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl PartialEq for LexItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LexItem::Operator(a), LexItem::Operator(b)) => a == b,
            (LexItem::SpecialChar(a), LexItem::SpecialChar(b)) => a == b,
            (LexItem::Operand(a), LexItem::Operand(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for LexItem {}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let result = self.peek();
        self.position += 1;
        result
    }

    fn read_identifier(&mut self) -> String {
        let mut result = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                result.push(self.next_char().unwrap());
            } else {
                break;
            }
        }

        result
    }

    fn read_string(&mut self) -> String {
        // Skip the opening quote
        self.next_char();

        let mut result = String::new();

        while let Some(c) = self.peek() {
            if c == '"' {
                // Skip the closing quote
                self.next_char();
                break;
            }
            result.push(self.next_char().unwrap());
        }

        result
    }

    pub fn read_number(&mut self) -> f64 {
        let mut result = String::new();
        let mut has_dot = false;

        match self.peek() {
            Some(c) if c == '-' => {
                result.push(self.next_char().unwrap());
            }
            _ => {}
        }

        while let Some(c) = self.peek() {
            match c {
                '0'..='9' => {
                    result.push(self.next_char().unwrap());
                }
                '.' if !has_dot => {
                    result.push(self.next_char().unwrap());
                    has_dot = true;
                }
                _ => break,
            }
        }

        result.parse().unwrap_or(0.0)
    }

    fn next_token(&mut self) -> Option<LexItem> {
        self.peek().map(|c| {
            match c {
                '(' | ')' | ',' | '.' => LexItem::SpecialChar(self.next_char().unwrap()),
                '"' => LexItem::Operand(Operand::Str(self.read_string())),
                '0'..='9' | '-' => LexItem::Operand(Operand::Num(self.read_number())),
                _ if c.is_alphabetic() => {
                    let ident = self.read_identifier();
                    match ident.as_str() {
                        // You would add other operators here
                        "eq" | "lt" | "gt" | "lte" | "gte" => LexItem::Operator(ident),
                        _ => LexItem::Operand(Operand::Str(ident)),
                    }
                }
                _ => panic!("Unexpected character when parsing!"),
            }
        })
    }

    pub fn tokenize(&mut self) -> Vec<LexItem> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            tokens.push(token);
        }
        tokens
    }
}

pub struct Parser {
    tokens: Vec<LexItem>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<LexItem>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    fn peek(&self) -> Option<&LexItem> {
        if self.position < self.tokens.len() {
            Some(&self.tokens[self.position])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<LexItem> {
        let token = self.peek().cloned();
        self.position += 1;
        token
    }

    fn operator_to_bson_key(operator: &String) -> Result<String, String> {
        match operator.as_str() {
            "eq" => Ok("$eq".to_string()),
            "lt" => Ok("$lt".to_string()),
            "gt" => Ok("$gt".to_string()),
            "lte" => Ok("$lte".to_string()),
            "gte" => Ok("$gte".to_string()),
            _ => Err(format!("Unknown operator: {}", operator)),
        }
    }

    fn parse_q_value_list(&mut self) -> Result<Vec<Bson>, String> {
        let mut q_values = Vec::new();

        while let Some(token) = self.advance() {
            match token {
                LexItem::SpecialChar(')') => break,
                LexItem::SpecialChar(',') => continue,
                LexItem::Operand(Operand::Str(fieldname)) => {
                    if let Some(LexItem::SpecialChar('.')) = self.advance() {
                        let bson_key = fieldname;
                        let bson_value = self.parse_q_value()?;
                        q_values.push(bson!({ bson_key: bson_value }));
                    }
                }
                _ => {
                    return Err(format!(
                        "Unexpected token at position {:?} | {:?}",
                        self.position, self.tokens
                    ));
                }
            }
        }

        Ok(q_values)
    }

    fn parse_q_value(&mut self) -> Result<Bson, String> {
        match self.peek() {
            Some(LexItem::Operator(_)) => match (self.advance(), self.advance(), self.advance()) {
                (
                    Some(LexItem::Operator(op)),
                    Some(LexItem::SpecialChar('.')),
                    Some(LexItem::Operand(value)),
                ) => {
                    let bson_key = Parser::operator_to_bson_key(&op)?;
                    let bson_value = match value {
                        Operand::Str(s) => Bson::String(s.clone()),
                        Operand::Num(n) => Bson::Double(n),
                    };
                    return Ok(bson!({ bson_key: bson_value }));
                }
                _ => {
                    return Err(format!(
                        "Unexpected token at position {:?} | {:?}",
                        self.position, self.tokens
                    ));
                }
            },
            Some(LexItem::SpecialChar('(')) => {
                self.advance();
                let bson_elements = self.parse_q_value_list()?;
                Ok(bson!(bson_elements))
            }
            _ => Err(format!(
                "Unexpected token at position {:?} | {:?}",
                self.position, self.tokens
            )),
        }
    }

    pub fn parse(&mut self) -> Result<Bson, String> {
        let result = self.parse_q_value()?;
        if self.peek() != None {
            return Err(format!(
                "Unexpected token at position {:?} | {:?}",
                self.position, self.tokens
            ));
        }
        Ok(result)
    }
}
