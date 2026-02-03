#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};

struct Unary {
    pub operand: Box<Op>,
}

struct Binary {
    pub left: Box<Op>,
    pub right: Box<Op>,
}

struct Condition {
    pub condition: Box<Op>,
    pub true_branch: Box<Op>,
    pub false_branch: Box<Op>,
}

enum Op {
    Constant(f64),
    Add(Binary),
    Sub(Binary),
    Div(Binary),
    Mul(Binary),
    Mod(Binary),
    Pow(Binary),
    BitwiseNot(Unary),
    BitwiseOr(Binary),
    BitwiseAnd(Binary),
    BitwiseShl(Binary),
    BitwiseShr(Binary),
    LogicalOr(Binary),
    LogicalAnd(Binary),
    Condition(Condition),
    Eq(Binary),
    Ne(Binary),
    Gt(Binary),
    Lt(Binary),
    Ge(Binary),
    Le(Binary),
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Identifier(String),
    LeftBracket,
    RightBracket,
    Then,
    Else,
    Add,
    Sub,
    Div,
    Mul,
    Mod,
    Pow,
    BitwiseNot,
    BitwiseOr,
    BitwiseAnd,
    BitwiseShl,
    BitwiseShr,
    LogicalOr,
    LogicalAnd,
    Condition,
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

impl Token {
    /// Returns true if the token is a special token
    /// used for control flow or grouping, or is a number or identifier.
    pub fn is_special(&self) -> bool {
        matches!(
            self,
            Token::LeftBracket
                | Token::RightBracket
                | Token::Then
                | Token::Else
                | Token::Condition
                | Token::Number(_)
                | Token::Identifier(_)
        )
    }

    /// Returns true if the token is left-associative i.e. it groups from the left.
    pub fn is_left_associative(&self) -> bool {
        matches!(
            self,
            Token::Add
                | Token::Sub
                | Token::Div
                | Token::Mul
                | Token::Mod
                | Token::BitwiseAnd
                | Token::BitwiseOr
                | Token::BitwiseShl
                | Token::BitwiseShr
                | Token::LogicalAnd
                | Token::LogicalOr
                | Token::Eq
                | Token::Ne
                | Token::Gt
                | Token::Lt
                | Token::Ge
                | Token::Le
        )
    }

    /// Returns the precedence of the token for operator precedence parsing.
    pub fn precedence(&self) -> u8 {
        match self {
            Token::LeftBracket => 0,
            Token::LogicalAnd | Token::LogicalOr => 1,
            Token::BitwiseAnd | Token::BitwiseOr => 2,
            Token::Eq | Token::Ne => 3,
            Token::Gt | Token::Lt | Token::Ge | Token::Le => 4,
            Token::BitwiseShl | Token::BitwiseShr => 5,
            Token::Add | Token::Sub => 6,
            Token::Mul | Token::Div | Token::Mod => 7,
            Token::Pow => 8,
            Token::BitwiseNot => 9,
            _ => 1, // Placeholder for other tokens
        }
    }
}

fn eval(op: &Op) -> f64 {
    match op {
        Op::Constant(n) => *n,
        Op::Add(b) => eval(&b.left) + eval(&b.right),
        Op::Sub(b) => eval(&b.left) - eval(&b.right),
        Op::Mul(b) => eval(&b.left) * eval(&b.right),
        Op::Div(b) => eval(&b.left) / eval(&b.right),
        Op::Mod(b) => eval(&b.left) % eval(&b.right),
        Op::Pow(b) => eval(&b.left).powf(eval(&b.right)),
        Op::BitwiseNot(u) => (!(eval(&u.operand) as i64)) as f64,
        Op::BitwiseAnd(b) => ((eval(&b.left) as i64) & (eval(&b.right) as i64)) as f64,
        Op::BitwiseOr(b) => ((eval(&b.left) as i64) | (eval(&b.right) as i64)) as f64,
        Op::BitwiseShl(b) => ((eval(&b.left) as i64) << (eval(&b.right) as i64)) as f64,
        Op::BitwiseShr(b) => ((eval(&b.left) as i64) >> (eval(&b.right) as i64)) as f64,
        Op::LogicalAnd(b) => {
            if eval(&b.left) != 0.0 && eval(&b.right) != 0.0 {
                1.0
            } else {
                0.0
            }
        }
        Op::LogicalOr(b) => {
            if eval(&b.left) != 0.0 || eval(&b.right) != 0.0 {
                1.0
            } else {
                0.0
            }
        }
        Op::Eq(b) => {
            if eval(&b.left) == eval(&b.right) {
                1.0
            } else {
                0.0
            }
        }
        Op::Ne(b) => {
            if eval(&b.left) != eval(&b.right) {
                1.0
            } else {
                0.0
            }
        }
        Op::Gt(b) => {
            if eval(&b.left) > eval(&b.right) {
                1.0
            } else {
                0.0
            }
        }
        Op::Lt(b) => {
            if eval(&b.left) < eval(&b.right) {
                1.0
            } else {
                0.0
            }
        }
        Op::Ge(b) => {
            if eval(&b.left) >= eval(&b.right) {
                1.0
            } else {
                0.0
            }
        }
        Op::Le(b) => {
            if eval(&b.left) <= eval(&b.right) {
                1.0
            } else {
                0.0
            }
        }
        Op::Condition(c) => {
            if eval(&c.condition) != 0.0 {
                eval(&c.true_branch)
            } else {
                eval(&c.false_branch)
            }
        }
    }
}

/// Evaluates a GenICam expression.
pub fn evaluate(expression: &str, variables: &HashMap<String, f64>) -> std::io::Result<f64> {
    let tokens = tokenize(expression)?;
    let op = parse(tokens, variables)?;
    Ok(eval(&op))
}

fn parse(tokens: Vec<Token>, variables: &HashMap<String, f64>) -> std::io::Result<Op> {
    let mut tokens = tokens.into_iter().collect::<VecDeque<Token>>();
    let mut operands = Vec::<Op>::new();
    let mut operators = Vec::<Token>::new();

    fn invalid_expression() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid expression")
    }

    fn pop_operand(operands: &mut Vec<Op>) -> std::io::Result<Op> {
        operands.pop().ok_or_else(invalid_expression)
    }

    fn apply_operator(operator: Token, operands: &mut Vec<Op>) -> std::io::Result<Op> {
        match operator {
            Token::Else => {
                let false_branch = pop_operand(operands)?;
                let true_branch = pop_operand(operands)?;
                let condition = pop_operand(operands)?;
                Ok(Op::Condition(Condition {
                    condition: Box::new(condition),
                    true_branch: Box::new(true_branch),
                    false_branch: Box::new(false_branch),
                }))
            }
            _ => make_op(operator, operands),
        }
    }

    while let Some(token) = tokens.pop_front() {
        match token {
            Token::Number(n) => {
                operands.push(Op::Constant(n));
            }
            Token::Identifier(i) => {
                if let Some(&value) = variables.get(&i) {
                    operands.push(Op::Constant(value));
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Undefined variable: {}", i),
                    ));
                }
            }
            Token::LeftBracket => {
                operators.push(Token::LeftBracket);
            }
            Token::RightBracket => {
                let mut found_left = false;
                while let Some(op) = operators.pop() {
                    if op == Token::LeftBracket {
                        found_left = true;
                        break;
                    }
                    let op_node = apply_operator(op, &mut operands)?;
                    operands.push(op_node);
                }
                if !found_left {
                    return Err(invalid_expression());
                }
            }
            Token::Then => {
                operators.push(Token::Then);
            }
            Token::Else => {
                // Pop operators until we find Then
                while let Some(op) = operators.last() {
                    if *op == Token::Then {
                        break;
                    }
                    let op = operators.pop().ok_or_else(invalid_expression)?;
                    let op_node = apply_operator(op, &mut operands)?;
                    operands.push(op_node);
                }
                if operators.last() != Some(&Token::Then) {
                    return Err(invalid_expression());
                }
                operators.pop(); // Remove Then
                let true_branch = pop_operand(&mut operands)?;
                let condition = pop_operand(&mut operands)?;

                // Now we need to parse the false branch
                // For simplicity, we'll push markers and handle later
                operands.push(condition);
                operands.push(true_branch);
                operators.push(Token::Else);
            }
            _ => {
                // This is an operator
                let current_op = token;

                // Pop operators with higher or equal precedence (for left-associative)
                while let Some(stack_op) = operators.last() {
                    if stack_op.is_special() && *stack_op != Token::BitwiseNot {
                        break;
                    }

                    let current_prec = current_op.precedence();
                    let stack_prec = stack_op.precedence();
                    let current_left_assoc = current_op.is_left_associative();

                    // Pop if:
                    // - stack operator has higher precedence, OR
                    // - same precedence and current is left-associative
                    let should_pop = if current_left_assoc {
                        stack_prec >= current_prec
                    } else {
                        stack_prec > current_prec
                    };

                    if should_pop {
                        let op = operators.pop().ok_or_else(invalid_expression)?;
                        let op_node = apply_operator(op, &mut operands)?;
                        operands.push(op_node);
                    } else {
                        break;
                    }
                }

                operators.push(current_op);
            }
        }
    }

    // Pop remaining operators
    while let Some(op) = operators.pop() {
        if op == Token::Then {
            return Err(invalid_expression());
        }
        let op_node = apply_operator(op, &mut operands)?;
        operands.push(op_node);
    }

    if operands.len() != 1 {
        return Err(invalid_expression());
    }

    pop_operand(&mut operands)
}

fn make_op(operator: Token, operands: &mut Vec<Op>) -> std::io::Result<Op> {
    fn invalid_expression() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid expression")
    }

    fn pop_operand(operands: &mut Vec<Op>) -> std::io::Result<Op> {
        operands.pop().ok_or_else(invalid_expression)
    }

    match operator {
        Token::Add => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Add(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Sub => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Sub(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Div => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Div(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Mul => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Mul(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Mod => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Mod(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Pow => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Pow(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::BitwiseNot => {
            let operand = pop_operand(operands)?;
            Ok(Op::BitwiseNot(Unary {
                operand: Box::new(operand),
            }))
        }
        Token::BitwiseAnd => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::BitwiseAnd(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::BitwiseOr => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::BitwiseOr(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::BitwiseShl => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::BitwiseShl(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::BitwiseShr => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::BitwiseShr(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::LogicalAnd => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::LogicalAnd(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::LogicalOr => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::LogicalOr(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Eq => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Eq(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Ne => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Ne(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Gt => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Gt(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Ge => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Ge(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Lt => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Lt(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        Token::Le => {
            let right = pop_operand(operands)?;
            let left = pop_operand(operands)?;
            Ok(Op::Le(Binary {
                left: Box::new(left),
                right: Box::new(right),
            }))
        }
        _ => Err(invalid_expression()),
    }
}

fn tokenize(expression: &str) -> std::io::Result<Vec<Token>> {
    let mut read_buffer = Vec::<char>::new();
    let mut chars = expression.chars().collect::<VecDeque<char>>();
    let mut tokens = Vec::<Token>::new();

    fn is_part_identifier(c: char, first: bool) -> bool {
        match c {
            '_' => true,
            _ => c.is_ascii_alphabetic() || (!first && c.is_ascii_digit()),
        }
    }
    fn is_part_base16(c: char) -> bool {
        c.is_digit(16)
    }
    fn is_part_base10(c: char, first: bool) -> bool {
        match c {
            '.' => true,
            '-' => !first,
            '+' => !first,
            'e' => !first,
            'E' => !first,
            _ => c.is_digit(10),
        }
    }

    while let Some(current) = chars.pop_front() {
        if is_part_base10(current, true) {
            read_buffer.clear();
            let is_base_16 = current == '0' && chars.front() == Some(&'x');
            if is_base_16 {
                chars.pop_front(); // Remove 'x'

                while let Some(&peek) = chars.front() {
                    if !is_part_base16(peek) {
                        break;
                    }

                    read_buffer.push(peek);
                    chars.pop_front();
                }

                let number = match u64::from_str_radix(
                    read_buffer.iter().collect::<String>().as_str(),
                    16,
                ) {
                    Ok(n) => n,
                    Err(_) => {
                        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                    }
                };

                tokens.push(Token::Number(number as f64));
                continue;
            } else {
                read_buffer.push(current);

                while let Some(&peek) = chars.front() {
                    if !is_part_base10(peek, false) {
                        break;
                    }

                    read_buffer.push(peek);
                    chars.pop_front();
                }

                let number = match read_buffer.iter().collect::<String>().parse::<f64>() {
                    Ok(n) => n,
                    Err(_) => {
                        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                    }
                };
                tokens.push(Token::Number(number));
                continue;
            }
        }

        if is_part_identifier(current, true) {
            read_buffer.clear();
            read_buffer.push(current);

            while let Some(&peek) = chars.front() {
                if !is_part_identifier(peek, false) {
                    break;
                }

                read_buffer.push(peek);
                chars.pop_front();
            }

            tokens.push(Token::Identifier(read_buffer.iter().collect()));
            continue;
        }

        match current {
            ' ' => continue,
            '+' => {
                tokens.push(Token::Add);
            }
            '-' => {
                tokens.push(Token::Sub);
            }
            '*' => {
                // Exponentation if next is also '*'
                if let Some(nc) = chars.front() {
                    if *nc == '*' {
                        chars.pop_front();
                        tokens.push(Token::Pow);
                        continue;
                    }
                }

                tokens.push(Token::Mul);
            }
            '/' => {
                tokens.push(Token::Div);
            }
            '^' => {
                tokens.push(Token::Pow);
            }
            '%' => {
                tokens.push(Token::Mod);
            }
            '|' => {
                // Logical OR if next is also '|'
                if let Some(nc) = chars.front() {
                    if *nc == '|' {
                        chars.pop_front();
                        tokens.push(Token::LogicalOr);
                        continue;
                    }
                }

                tokens.push(Token::BitwiseOr);
            }
            '&' => {
                // Logical AND if next is also '&'
                if let Some(nc) = chars.front() {
                    if *nc == '&' {
                        chars.pop_front();
                        tokens.push(Token::LogicalAnd);
                        continue;
                    }
                }

                tokens.push(Token::BitwiseAnd);
            }
            '~' => {
                tokens.push(Token::BitwiseNot);
            }
            '(' => {
                tokens.push(Token::LeftBracket);
            }
            ')' => {
                tokens.push(Token::RightBracket);
            }
            '<' => {
                if let Some(nc) = chars.front() {
                    match nc {
                        '=' => {
                            chars.pop_front();
                            tokens.push(Token::Le);
                            continue;
                        }
                        '<' => {
                            chars.pop_front();
                            tokens.push(Token::BitwiseShl);
                            continue;
                        }
                        '>' => {
                            chars.pop_front();
                            tokens.push(Token::Ne);
                            continue;
                        }
                        _ => {}
                    }
                }
                tokens.push(Token::Lt);
            }
            '>' => {
                if let Some(nc) = chars.front() {
                    match nc {
                        '=' => {
                            chars.pop_front();
                            tokens.push(Token::Ge);
                            continue;
                        }
                        '>' => {
                            chars.pop_front();
                            tokens.push(Token::BitwiseShr);
                            continue;
                        }
                        _ => {}
                    }
                }
                tokens.push(Token::Gt);
            }
            '=' => {
                tokens.push(Token::Eq);
            }
            '?' => {
                tokens.push(Token::Then);
            }
            ':' => {
                tokens.push(Token::Else);
            }
            _ => {
                // Handle other characters or return an error
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_numbers(tokens: &[Token]) -> Vec<f64> {
        tokens
            .iter()
            .filter_map(|t| match t {
                Token::Number(n) => Some(*n),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_hexadecimal_numbers() {
        let result = tokenize("0xffff").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![65535.0]);

        let result = tokenize("0x10").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![16.0]);

        let result = tokenize("0xABCD").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![43981.0]);
    }

    #[test]
    fn test_decimal_integers() {
        let result = tokenize("1239834").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![1239834.0]);

        let result = tokenize("0").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![0.0]);

        let result = tokenize("42").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![42.0]);
    }

    #[test]
    fn test_floating_point_numbers() {
        let result = tokenize("1.234").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![1.234]);

        let result = tokenize("0.5").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![0.5]);

        let result = tokenize("123.456789").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![123.456789]);
    }

    #[test]
    fn test_scientific_notation_positive_exponent() {
        let result = tokenize("1e10").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![1e10]);

        let result = tokenize("1e+10").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![1e+10]);

        let result = tokenize("2.5e5").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![2.5e5]);

        let result = tokenize("3.14e+2").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![3.14e+2]);
    }

    #[test]
    fn test_scientific_notation_negative_exponent() {
        let result = tokenize("1e-10").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![1e-10]);

        let result = tokenize("2.5e-3").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![2.5e-3]);

        let result = tokenize("1.23e-5").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![1.23e-5]);
    }

    #[test]
    fn test_multiple_numbers() {
        let result = tokenize("1.234 0xffff 42 1e-10").expect("tokenize failed");
        let numbers = extract_numbers(&result);
        assert_eq!(numbers, vec![1.234, 65535.0, 42.0, 1e-10]);
    }

    fn filter_tokens<F>(tokens: &[Token], predicate: F) -> Vec<Token>
    where
        F: Fn(&Token) -> bool,
    {
        tokens.iter().filter(|t| predicate(t)).cloned().collect()
    }

    #[test]
    fn test_arithmetic_operators() {
        let result = tokenize("1 + 2").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::Number(1.0), Token::Add, Token::Number(2.0),]
        );

        let result = tokenize("5 - 3").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::Number(5.0), Token::Sub, Token::Number(3.0),]
        );

        let result = tokenize("4 * 2").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::Number(4.0), Token::Mul, Token::Number(2.0),]
        );

        let result = tokenize("10 / 2").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::Number(10.0), Token::Div, Token::Number(2.0),]
        );

        let result = tokenize("10 % 3").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::Number(10.0), Token::Mod, Token::Number(3.0),]
        );
    }

    #[test]
    fn test_exponentiation_operators() {
        let result = tokenize("2 ** 3").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::Number(2.0), Token::Pow, Token::Number(3.0),]
        );

        let result = tokenize("2 ^ 3").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::Number(2.0), Token::Pow, Token::Number(3.0),]
        );
    }

    #[test]
    fn test_bitwise_operators() {
        let result = tokenize("a & b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::BitwiseAnd,
                Token::Identifier("b".to_string()),
            ]
        );

        let result = tokenize("a | b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::BitwiseOr,
                Token::Identifier("b".to_string()),
            ]
        );

        let result = tokenize("a << 2").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::BitwiseShl,
                Token::Number(2.0),
            ]
        );

        let result = tokenize("a >> 2").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::BitwiseShr,
                Token::Number(2.0),
            ]
        );

        let result = tokenize("~a").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::BitwiseNot, Token::Identifier("a".to_string()),]
        );
    }

    #[test]
    fn test_logical_operators() {
        let result = tokenize("a && b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::LogicalAnd,
                Token::Identifier("b".to_string()),
            ]
        );

        let result = tokenize("a || b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::LogicalOr,
                Token::Identifier("b".to_string()),
            ]
        );
    }

    #[test]
    fn test_comparison_operators() {
        let result = tokenize("a = b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::Eq,
                Token::Identifier("b".to_string()),
            ]
        );

        let result = tokenize("a <> b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::Ne,
                Token::Identifier("b".to_string()),
            ]
        );

        let result = tokenize("a < b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::Lt,
                Token::Identifier("b".to_string()),
            ]
        );

        let result = tokenize("a > b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::Gt,
                Token::Identifier("b".to_string()),
            ]
        );

        let result = tokenize("a <= b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::Le,
                Token::Identifier("b".to_string()),
            ]
        );

        let result = tokenize("a >= b").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::Ge,
                Token::Identifier("b".to_string()),
            ]
        );
    }

    #[test]
    fn test_parentheses() {
        let result = tokenize("(a + b)").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::LeftBracket,
                Token::Identifier("a".to_string()),
                Token::Add,
                Token::Identifier("b".to_string()),
                Token::RightBracket,
            ]
        );
    }

    #[test]
    fn test_ternary_conditional() {
        let result = tokenize("a ? b : c").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("a".to_string()),
                Token::Then,
                Token::Identifier("b".to_string()),
                Token::Else,
                Token::Identifier("c".to_string()),
            ]
        );
    }

    #[test]
    fn test_identifiers() {
        let result = tokenize("variable_name").expect("tokenize failed");
        assert_eq!(
            result,
            vec![Token::Identifier("variable_name".to_string()),]
        );

        let result = tokenize("CamelCase").expect("tokenize failed");
        assert_eq!(result, vec![Token::Identifier("CamelCase".to_string()),]);

        let result = tokenize("_private123").expect("tokenize failed");
        assert_eq!(result, vec![Token::Identifier("_private123".to_string()),]);
    }

    #[test]
    fn test_complex_expression() {
        let result = tokenize("(a + b) * c - d / 2.5 % 3").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::LeftBracket,
                Token::Identifier("a".to_string()),
                Token::Add,
                Token::Identifier("b".to_string()),
                Token::RightBracket,
                Token::Mul,
                Token::Identifier("c".to_string()),
                Token::Sub,
                Token::Identifier("d".to_string()),
                Token::Div,
                Token::Number(2.5),
                Token::Mod,
                Token::Number(3.0),
            ]
        );
    }

    #[test]
    fn test_mixed_operators_and_identifiers() {
        let result = tokenize("x * 2 + y >> 1 & ~z").expect("tokenize failed");
        assert_eq!(
            result,
            vec![
                Token::Identifier("x".to_string()),
                Token::Mul,
                Token::Number(2.0),
                Token::Add,
                Token::Identifier("y".to_string()),
                Token::BitwiseShr,
                Token::Number(1.0),
                Token::BitwiseAnd,
                Token::BitwiseNot,
                Token::Identifier("z".to_string()),
            ]
        );
    }

    #[test]
    fn test_whitespace_handling() {
        let result1 = tokenize("a+b").expect("tokenize failed");
        let result2 = tokenize("a + b").expect("tokenize failed");
        let result3 = tokenize("a   +   b").expect("tokenize failed");

        let expected = vec![
            Token::Identifier("a".to_string()),
            Token::Add,
            Token::Identifier("b".to_string()),
        ];

        assert_eq!(result1, expected);
        assert_eq!(result2, expected);
        assert_eq!(result3, expected);
    }

    #[test]
    fn test_parse_simple_constant() {
        let tokens = tokenize("42").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 42.0);
    }

    #[test]
    fn test_parse_simple_addition() {
        let tokens = tokenize("1 + 2").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 3.0);
    }

    #[test]
    fn test_parse_simple_subtraction() {
        let tokens = tokenize("5 - 3").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 2.0);
    }

    #[test]
    fn test_parse_simple_multiplication() {
        let tokens = tokenize("4 * 2").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 8.0);
    }

    #[test]
    fn test_parse_simple_division() {
        let tokens = tokenize("10 / 2").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 5.0);
    }

    #[test]
    fn test_parse_precedence_mul_over_add() {
        let tokens = tokenize("1 + 2 * 3").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 7.0); // 1 + (2 * 3)
    }

    #[test]
    fn test_parse_precedence_div_over_sub() {
        let tokens = tokenize("10 - 6 / 2").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 7.0); // 10 - (6 / 2)
    }

    #[test]
    fn test_parse_left_associativity() {
        let tokens = tokenize("10 - 3 - 2").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 5.0); // (10 - 3) - 2 = 5
    }

    #[test]
    fn test_parse_parentheses() {
        let tokens = tokenize("(1 + 2) * 3").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 9.0);
    }

    #[test]
    fn test_parse_nested_parentheses() {
        let tokens = tokenize("((1 + 2) * 3) + 4").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 13.0);
    }

    #[test]
    fn test_parse_power_operator() {
        let tokens = tokenize("2 ** 3").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 8.0);
    }

    #[test]
    fn test_parse_modulo_operator() {
        let tokens = tokenize("10 % 3").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);
    }

    #[test]
    fn test_parse_bitwise_and() {
        let tokens = tokenize("12 & 10").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 8.0); // 1100 & 1010 = 1000
    }

    #[test]
    fn test_parse_bitwise_or() {
        let tokens = tokenize("12 | 10").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 14.0); // 1100 | 1010 = 1110
    }

    #[test]
    fn test_parse_bitwise_shift_left() {
        let tokens = tokenize("5 << 2").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 20.0);
    }

    #[test]
    fn test_parse_bitwise_shift_right() {
        let tokens = tokenize("20 >> 2").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 5.0);
    }

    #[test]
    fn test_parse_bitwise_not() {
        let tokens = tokenize("~5").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), -6.0);
    }

    #[test]
    fn test_parse_logical_and() {
        let tokens = tokenize("1 && 1").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("1 && 0").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 0.0);
    }

    #[test]
    fn test_parse_logical_or() {
        let tokens = tokenize("0 || 1").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("0 || 0").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 0.0);
    }

    #[test]
    fn test_parse_comparison_equal() {
        let tokens = tokenize("5 = 5").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("5 = 3").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 0.0);
    }

    #[test]
    fn test_parse_comparison_not_equal() {
        let tokens = tokenize("5 <> 3").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("5 <> 5").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 0.0);
    }

    #[test]
    fn test_parse_comparison_greater_than() {
        let tokens = tokenize("5 > 3").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("3 > 5").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 0.0);
    }

    #[test]
    fn test_parse_comparison_less_than() {
        let tokens = tokenize("3 < 5").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("5 < 3").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 0.0);
    }

    #[test]
    fn test_parse_comparison_greater_equal() {
        let tokens = tokenize("5 >= 5").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("5 >= 3").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("3 >= 5").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 0.0);
    }

    #[test]
    fn test_parse_comparison_less_equal() {
        let tokens = tokenize("3 <= 5").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("5 <= 5").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 1.0);

        let tokens = tokenize("5 <= 3").expect("tokenize failed");
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 0.0);
    }

    #[test]
    fn test_parse_ternary_condition_true() {
        let tokens = tokenize("1 ? 42 : 99").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 42.0);
    }

    #[test]
    fn test_parse_ternary_condition_false() {
        let tokens = tokenize("0 ? 42 : 99").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 99.0);
    }

    #[test]
    fn test_parse_variable_substitution() {
        let tokens = tokenize("x + y").expect("tokenize failed");
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), 10.0);
        vars.insert("y".to_string(), 20.0);
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 30.0);
    }

    #[test]
    fn test_parse_undefined_variable() {
        let tokens = tokenize("x + y").expect("tokenize failed");
        let vars = HashMap::new();
        let result = parse(tokens, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_complex_expression() {
        let tokens = tokenize("(1 + 2) * 3 - 4 / 2").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 7.0); // (1 + 2) * 3 - 4 / 2 = 9 - 2 = 7
    }

    #[test]
    fn test_parse_mixed_operators() {
        let tokens = tokenize("10 + 5 * 2 - 8 / 4").expect("tokenize failed");
        let vars = HashMap::new();
        let op = parse(tokens, &vars).expect("parse failed");
        assert_eq!(eval(&op), 18.0); // 10 + 10 - 2 = 18
    }

    #[test]
    fn test_parse_invalid_missing_operand() {
        let tokens = tokenize("1 +").expect("tokenize failed");
        let vars = HashMap::new();
        let result = parse(tokens, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_unmatched_left_paren() {
        let tokens = tokenize("(1 + 2").expect("tokenize failed");
        let vars = HashMap::new();
        let result = parse(tokens, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_unmatched_right_paren() {
        let tokens = tokenize("1 + 2)").expect("tokenize failed");
        let vars = HashMap::new();
        let result = parse(tokens, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_empty_expression() {
        let tokens = tokenize("").expect("tokenize failed");
        let vars = HashMap::new();
        let result = parse(tokens, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_ternary_missing_else() {
        let tokens = tokenize("1 ? 2").expect("tokenize failed");
        let vars = HashMap::new();
        let result = parse(tokens, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_ternary_missing_then() {
        let tokens = tokenize("1 : 2").expect("tokenize failed");
        let vars = HashMap::new();
        let result = parse(tokens, &vars);
        assert!(result.is_err());
    }
}
