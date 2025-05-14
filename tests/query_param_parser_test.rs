#[cfg(test)]
mod tests {
    use mongodb::bson::bson;
    use mongor::query_param_parser::{LexItem, Lexer, Operand, Parser};

    #[test]
    fn test_read_number() {
        let test_cases = [
            ("123", 123.0),
            ("42.5", 42.5),
            ("0.123", 0.123),
            ("123rest", 123.0), // Should read only the number
            (".123", 0.123),
            ("-.123", -0.123),
            ("-5.123", -5.123),
        ];

        // println!("Parsed: {}", ".123".parse::<f64>().expect("Should work"));
        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            println!("Lexer input: {}", input);
            assert_eq!(lexer.read_number(), expected);
        }
    }

    #[test]
    fn test_tokenize_empty() {
        let mut lexer = Lexer::new("");
        assert_eq!(lexer.tokenize(), vec![]);
    }

    #[test]
    fn test_tokenize_punctuation() {
        let mut lexer = Lexer::new(".,()");
        assert_eq!(
            lexer.tokenize(),
            vec![
                LexItem::SpecialChar('.'),
                LexItem::SpecialChar(','),
                LexItem::SpecialChar('('),
                LexItem::SpecialChar(')'),
            ]
        );
    }

    #[test]
    fn test_tokenize_values() {
        let test_cases = [
            (
                "eq.2.05",
                vec![
                    LexItem::Operator("eq".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Operand(Operand::Num(2.05)),
                ],
            ),
            (
                "(field1.eq.\"test\",field2.lt.24.5,field3.gte.-2,field4.eq.hello)",
                vec![
                    LexItem::SpecialChar('('),
                    LexItem::Operand(Operand::Str("field1".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::Operator("eq".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Operand(Operand::Str(("test".to_string()))),
                    LexItem::SpecialChar(','),
                    LexItem::Operand(Operand::Str("field2".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::Operator("lt".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Operand(Operand::Num(24.5)),
                    LexItem::SpecialChar(','),
                    LexItem::Operand(Operand::Str("field3".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::Operator("gte".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Operand(Operand::Num(-2.0)),
                    LexItem::SpecialChar(','),
                    LexItem::Operand(Operand::Str("field4".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::Operator("eq".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Operand(Operand::Str("hello".to_string())),
                    LexItem::SpecialChar(')'),
                ],
            ),
        ];
        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            assert_eq!(lexer.tokenize(), expected);
        }
    }

    #[test]
    fn test_values_values() {
        let test_cases = [
            (
                "eq.2.05",
                bson!({
                    "$eq": 2.05
                }),
            ),
            (
                "(field1.eq.\"test\",field2.lt.24.5,field3.gte.-2,field4.eq.hello)",
                bson!([
                    {
                        "field1": {
                            "$eq": "test"
                        }
                    },
                    {
                        "field2": {
                            "$lt": 24.5
                        }
                    },
                    {
                        "field3": {
                            "$gte": -2.0
                        }
                    },
                    {
                        "field4": {
                            "$eq": "hello"
                        }
                    }
                ]),
            ),
            (
                "(\"a.b.c\".eq.test)",
                bson!([
                    {
                        "a.b.c": {
                            "$eq": "test"
                        }
                    }
                ]),
            ),
        ];
        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer.tokenize());
            assert_eq!(parser.parse().expect("Not good"), expected);
        }
    }
}
