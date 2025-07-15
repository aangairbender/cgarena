use std::{fmt::Display, str::FromStr};

use anyhow::{anyhow, bail};

use crate::domain::{Match, MatchAttribute, MatchAttributeValue};

#[derive(Clone)]
pub struct MatchFilter {
    expr: Option<ast::Expr>,
}

impl MatchFilter {
    pub fn accept_all() -> Self {
        Self { expr: None }
    }

    pub fn matches(&self, m: &Match) -> bool {
        if let Some(ref expr) = self.expr {
            check_expr(expr, m).unwrap_or(false)
        } else {
            true
        }
    }

    pub fn needed_attributes(&self) -> Vec<MatchAttribute> {
        let mut res = vec![];
        if let Some(ref expr) = self.expr {
            collect_needed_attributes(expr, &mut res);
        }
        res
    }
}

fn collect_needed_attributes(expr: &ast::Expr, res: &mut Vec<MatchAttribute>) {
    match expr {
        ast::Expr::Condition(argument1, _, argument2) => {
            collect_needed_attributes_from_arg(argument1, res);
            collect_needed_attributes_from_arg(argument2, res);
        }
        ast::Expr::Paren(expr) => collect_needed_attributes(expr, res),
        ast::Expr::And(expr1, expr2) => {
            collect_needed_attributes(expr1, res);
            collect_needed_attributes(expr2, res);
        }
        ast::Expr::Or(expr1, expr2) => {
            collect_needed_attributes(expr1, res);
            collect_needed_attributes(expr2, res);
        }
        ast::Expr::Not(expr) => collect_needed_attributes(expr, res),
    }
}

fn collect_needed_attributes_from_arg(arg: &ast::Argument, res: &mut Vec<MatchAttribute>) {
    match arg {
        ast::Argument::Value(_) => {}
        ast::Argument::MatchAttr(match_attr) => {
            let attr = MatchAttribute {
                name: match_attr.name.clone(),
                bot_id: None,
                turn: match_attr.turn,
                // don't care about value
                value: MatchAttributeValue::Integer(0),
            };
            res.push(attr);
        }
        ast::Argument::BotAttr(bot_attr) => {
            let attr = MatchAttribute {
                name: bot_attr.name.clone(),
                bot_id: Some(bot_attr.bot_id),
                turn: bot_attr.turn,
                // don't care about value
                value: MatchAttributeValue::Integer(0),
            };
            res.push(attr);
        }
    }
}

fn check_expr(expr: &ast::Expr, m: &Match) -> Result<bool, anyhow::Error> {
    match expr {
        ast::Expr::Condition(arg1, op, arg2) => check_condition(arg1, op, arg2, m),
        ast::Expr::Paren(expr) => check_expr(expr, m),
        ast::Expr::And(expr1, expr2) => Ok(check_expr(expr1, m)? && check_expr(expr2, m)?),
        ast::Expr::Or(expr1, expr2) => Ok(check_expr(expr1, m)? || check_expr(expr2, m)?),
        ast::Expr::Not(expr) => Ok(!check_expr(expr, m)?),
    }
}

fn check_condition(
    arg1: &ast::Argument,
    op: &ast::ConditionOp,
    arg2: &ast::Argument,
    m: &Match,
) -> Result<bool, anyhow::Error> {
    let arg1 = match arg1 {
        ast::Argument::Value(value) => &value.clone().into(),
        ast::Argument::MatchAttr(attr) => extract_match_attr(m, attr)?,
        ast::Argument::BotAttr(attr) => extract_bot_attr(m, attr)?,
    };
    let arg2 = match arg2 {
        ast::Argument::Value(value) => &value.clone().into(),
        ast::Argument::MatchAttr(attr) => extract_match_attr(m, attr)?,
        ast::Argument::BotAttr(attr) => extract_bot_attr(m, attr)?,
    };

    let arg1 = if let MatchAttributeValue::Integer(v) = arg1 {
        &MatchAttributeValue::Float(*v as f64)
    } else {
        arg1
    };

    let arg2 = if let MatchAttributeValue::Integer(v) = arg2 {
        &MatchAttributeValue::Float(*v as f64)
    } else {
        arg2
    };

    let res = match (arg1, arg2) {
        (MatchAttributeValue::Float(a1), MatchAttributeValue::Float(a2)) => match op {
            ast::ConditionOp::Eq => a1 == a2,
            ast::ConditionOp::NotEq => a1 != a2,
            ast::ConditionOp::Less => a1 < a2,
            ast::ConditionOp::LessOrEqual => a1 <= a2,
            ast::ConditionOp::Greater => a1 > a2,
            ast::ConditionOp::GreaterOrEqual => a1 >= a2,
        },
        (MatchAttributeValue::String(a1), MatchAttributeValue::String(a2)) => match op {
            ast::ConditionOp::Eq => a1 == a2,
            ast::ConditionOp::NotEq => a1 != a2,
            _ => bail!("Operator {op} is not applicable to strings"),
        },
        _ => bail!("Type mismatch"),
    };
    Ok(res)
}

fn extract_match_attr<'m>(
    m: &'m Match,
    attr: &ast::MatchAttr,
) -> Result<&'m MatchAttributeValue, anyhow::Error> {
    m.attributes
        .iter()
        .find(|a| a.name == attr.name && a.turn == attr.turn)
        .map(|a| &a.value)
        .ok_or(anyhow!("No such attribute"))
}

fn extract_bot_attr<'m>(
    m: &'m Match,
    attr: &ast::BotAttr,
) -> Result<&'m MatchAttributeValue, anyhow::Error> {
    m.attributes
        .iter()
        .find(|a| a.name == attr.name && a.turn == attr.turn && a.bot_id == Some(attr.bot_id))
        .map(|a| &a.value)
        .ok_or(anyhow!("No such attribute"))
}

impl From<ast::Value> for MatchAttributeValue {
    fn from(value: ast::Value) -> Self {
        match value {
            ast::Value::Number(v) => MatchAttributeValue::Float(v),
            ast::Value::String(v) => MatchAttributeValue::String(v),
        }
    }
}

impl FromStr for MatchFilter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Self::accept_all())
        } else {
            ast::parse(s).map(|expr| Self { expr: Some(expr) })
        }
    }
}

impl Display for MatchFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref expr) = self.expr {
            write!(f, "{}", expr)
        } else {
            write!(f, "")
        }
    }
}

mod ast {
    use std::fmt::Display;

    use nom::{
        branch::alt,
        bytes::complete::{is_not, tag, tag_no_case},
        character::complete::{self, alpha1, alphanumeric1, multispace0},
        combinator::{map, opt, recognize},
        multi::{many, many0_count},
        number,
        sequence::{delimited, pair, preceded},
        IResult, Parser,
    };

    use crate::domain::BotId;

    #[derive(Clone)]
    pub enum Expr {
        Condition(Argument, ConditionOp, Argument),
        Paren(Box<Expr>),
        And(Box<Expr>, Box<Expr>),
        Or(Box<Expr>, Box<Expr>),
        Not(Box<Expr>),
    }

    #[derive(Clone)]
    pub enum Argument {
        Value(Value),
        MatchAttr(MatchAttr),
        BotAttr(BotAttr),
    }

    #[derive(Clone)]
    pub enum ConditionOp {
        Eq,
        NotEq,
        Less,
        LessOrEqual,
        Greater,
        GreaterOrEqual,
    }

    #[derive(Clone)]
    pub enum Value {
        Number(f64),
        String(String),
    }

    #[derive(Clone)]
    pub struct MatchAttr {
        pub name: String,
        pub turn: Option<u16>,
    }

    #[derive(Clone)]
    pub struct BotAttr {
        pub name: String,
        pub turn: Option<u16>,
        pub bot_id: BotId,
    }

    impl Display for ConditionOp {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ConditionOp::Eq => write!(f, "=="),
                ConditionOp::NotEq => write!(f, "!="),
                ConditionOp::Less => write!(f, "<"),
                ConditionOp::LessOrEqual => write!(f, "<="),
                ConditionOp::Greater => write!(f, ">"),
                ConditionOp::GreaterOrEqual => write!(f, ">="),
            }
        }
    }

    impl Display for Value {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Value::Number(v) => write!(f, "{}", v),
                Value::String(v) => write!(f, "\"{}\"", v),
            }
        }
    }

    impl Display for MatchAttr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(turn) = self.turn {
                write!(f, "match[{}].{}", turn, self.name)
            } else {
                write!(f, "match.{}", self.name)
            }
        }
    }

    impl Display for BotAttr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(turn) = self.turn {
                write!(f, "bot({})[{}].{}", self.bot_id, turn, self.name)
            } else {
                write!(f, "bot({}).{}", self.bot_id, self.name)
            }
        }
    }

    impl Display for Argument {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Argument::Value(value) => write!(f, "{}", value),
                Argument::MatchAttr(match_attr) => write!(f, "{}", match_attr),
                Argument::BotAttr(bot_attr) => write!(f, "{}", bot_attr),
            }
        }
    }

    impl Display for Expr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match *self {
                Expr::Condition(ref arg1, ref op, ref arg2) => {
                    write!(f, "{} {} {}", arg1, op, arg2)
                }
                Expr::Paren(ref expr) => write!(f, "({})", expr),
                Expr::And(ref expr1, ref expr2) => write!(f, "{} AND {}", expr1, expr2),
                Expr::Or(ref expr1, ref expr2) => write!(f, "{} OR {}", expr1, expr2),
                Expr::Not(ref expr) => write!(f, "NOT {}", expr),
            }
        }
    }

    pub fn parse(input: &str) -> Result<Expr, anyhow::Error> {
        let (remaining, expr) = expression(input)
            .map_err(|_| anyhow::anyhow!("Invalid filter syntax, please check docs"))?;
        if remaining.is_empty() {
            Ok(expr)
        } else {
            Err(anyhow::anyhow!("Unexpected suffix: {}", remaining))
        }
    }

    fn expression(i: &str) -> IResult<&str, Expr> {
        let (i, initial) = term(i)?;
        let (i, remainder) = many(0.., |i| {
            let (i, sub) = preceded(tag_no_case("or"), term).parse(i)?;
            Ok((i, (Oper::Or, sub)))
        })
        .parse(i)?;

        Ok((i, fold_exprs(initial, remainder)))
    }

    fn term(i: &str) -> IResult<&str, Expr> {
        let (i, initial) = factor(i)?;
        let (i, remainder) = many(0.., |i| {
            let (i, sub) = preceded(tag_no_case("and"), factor).parse(i)?;
            Ok((i, (Oper::And, sub)))
        })
        .parse(i)?;

        Ok((i, fold_exprs(initial, remainder)))
    }

    fn factor(i: &str) -> IResult<&str, Expr> {
        map(
            (
                opt(delimited(multispace0, tag_no_case("not"), multispace0)),
                factor_inner,
            ),
            |(not, f)| {
                if not.is_some() {
                    Expr::Not(Box::new(f))
                } else {
                    f
                }
            },
        )
        .parse(i)
    }

    fn factor_inner(i: &str) -> IResult<&str, Expr> {
        alt((condition, parens)).parse(i)
    }

    fn condition(i: &str) -> IResult<&str, Expr> {
        map((argument, condition_op, argument), |(arg1, op, arg2)| {
            Expr::Condition(arg1, op, arg2)
        })
        .parse(i)
    }

    fn argument(i: &str) -> IResult<&str, Argument> {
        delimited(
            multispace0,
            alt((match_attr, bot_attr, just_value)),
            multispace0,
        )
        .parse(i)
    }

    fn condition_op(i: &str) -> IResult<&str, ConditionOp> {
        alt((
            map(tag("=="), |_| ConditionOp::Eq),
            map(tag("!="), |_| ConditionOp::NotEq),
            map(tag("<="), |_| ConditionOp::LessOrEqual),
            map(tag(">="), |_| ConditionOp::GreaterOrEqual),
            map(tag("<"), |_| ConditionOp::Less),
            map(tag(">"), |_| ConditionOp::Greater),
        ))
        .parse(i)
    }

    fn just_value(i: &str) -> IResult<&str, Argument> {
        alt((
            map(number::complete::double, |v| {
                Argument::Value(Value::Number(v))
            }),
            delimited(
                tag("\""),
                map(is_not("\""), |s: &str| {
                    Argument::Value(Value::String(s.to_string()))
                }),
                tag("\""),
            ),
        ))
        .parse(i)
    }

    fn fold_exprs(initial: Expr, remainder: Vec<(Oper, Expr)>) -> Expr {
        remainder.into_iter().fold(initial, |acc, pair| {
            let (oper, expr) = pair;
            match oper {
                Oper::And => Expr::And(Box::new(acc), Box::new(expr)),
                Oper::Or => Expr::Or(Box::new(acc), Box::new(expr)),
            }
        })
    }

    enum Oper {
        And,
        Or,
    }

    fn parens(i: &str) -> IResult<&str, Expr> {
        delimited(
            multispace0,
            delimited(
                tag("("),
                map(expression, |e| Expr::Paren(Box::new(e))),
                tag(")"),
            ),
            multispace0,
        )
        .parse(i)
    }

    fn match_attr(i: &str) -> IResult<&str, Argument> {
        let (remaining, w) = (
            tag_no_case("match"),
            opt(delimited(tag("["), complete::u16, tag("]"))),
            tag("."),
            identifier,
        )
            .parse(i)?;

        Ok((
            remaining,
            Argument::MatchAttr(MatchAttr {
                turn: w.1,
                name: w.3.to_string(),
            }),
        ))
    }

    fn bot_attr(i: &str) -> IResult<&str, Argument> {
        let (remaining, w) = (
            tag_no_case("bot"),
            delimited(tag("("), complete::i64, tag(")")),
            opt(delimited(tag("["), complete::u16, tag("]"))),
            tag("."),
            identifier,
        )
            .parse(i)?;

        Ok((
            remaining,
            Argument::BotAttr(BotAttr {
                bot_id: w.1.into(),
                turn: w.2,
                name: w.4.to_string(),
            }),
        ))
    }

    fn identifier(input: &str) -> IResult<&str, &str> {
        recognize(pair(
            alt((alpha1, tag("_"))),
            many0_count(alt((alphanumeric1, tag("_")))),
        ))
        .parse(input)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{BotId, MatchAttribute};

    use super::*;

    macro_rules! generate_query_tests {
        ($($name:ident: $query:expr),* $(,)?) => {
            $(
                #[test]
                fn $name() {
                    let expr = ast::parse($query).unwrap();
                    let displayed = format!("{}", expr);
                    assert_eq!($query, displayed);
                }
            )*
        };
    }

    generate_query_tests!(
        arithmetic: "1 == 2",
        player_count_eq_2: "match.player_count == 2",
        some_data_neq: "match[5].some_data != -2",
        bot_qq_gt: "bot(23).qq > 5",
        bot_qwe_ge: "bot(1)[50].qwe >= 100",
        match_lt: "match.a < 100",
        match_le: "match.a <= 100",
        match_eq_str: "match.www == \"asd\"",
        match_neq_str: "match.www != \"asd\"",
        and_combined: "match.a == 1 AND match.b == 2 AND match.c == 3",
        or_combined: "match.a == 5 OR match.b == 2 OR match.c == 3",
        not_eq: "NOT match.coins == 5",
        not_and_combined: "NOT match.a == 1 AND NOT match.b == 2 AND NOT match.c == 3",
        not_or_combined: "NOT match.a == 5 OR NOT match.b == 2 OR NOT match.c == 3",
        parens_single: "(match.a < 100)",
        parens_nested: "(((match.a < 100)))",
        and_or_grouped: "match.a == 2 AND (match.x > 1 OR match.y < 1)",
        or_and_grouped: "match.a == 2 OR (match.x > 1 AND match.y < 1)",
        match_eq_match: "match.a == match.b",
    );

    #[test]
    fn doubles() {
        let expr = ast::parse("1.0==2.0").unwrap();
        let displayed = format!("{}", expr);
        assert_eq!("1 == 2", displayed);
    }

    #[test]
    fn spaced() {
        let expr = ast::parse(
            "  (  match.a == 1  OR  match.a  ==  2 )  AND ( match.x  ==  1  OR  match.y  ==  1 ) ",
        )
        .unwrap();
        let displayed = format!("{}", expr);
        assert_eq!(
            "(match.a == 1 OR match.a == 2) AND (match.x == 1 OR match.y == 1)",
            displayed
        );
    }

    #[test]
    fn case_sensitivity() {
        let expr = ast::parse("1 == 2 and not 2 == 5").unwrap();
        let displayed = format!("{}", expr);
        assert_eq!("1 == 2 AND NOT 2 == 5", displayed);
    }

    #[test]
    fn filtering() {
        let bot_id1: BotId = 1i64.into();
        let bot_id2: BotId = 2i64.into();
        let bot_id3: BotId = 3i64.into();

        let attributes = vec![
            MatchAttribute {
                name: "initial_stones".to_string(),
                bot_id: None,
                turn: None,
                value: "25".to_string().into(),
            },
            MatchAttribute {
                name: "map_type".to_string(),
                bot_id: None,
                turn: None,
                value: "small".to_string().into(),
            },
            MatchAttribute {
                name: "stones_percentage".to_string(),
                bot_id: None,
                turn: None,
                value: "0.75".to_string().into(),
            },
            MatchAttribute {
                name: "final_score".to_string(),
                bot_id: Some(bot_id1),
                turn: None,
                value: "75".to_string().into(),
            },
            MatchAttribute {
                name: "final_score".to_string(),
                bot_id: Some(bot_id2),
                turn: None,
                value: "50".to_string().into(),
            },
            MatchAttribute {
                name: "score".to_string(),
                bot_id: Some(bot_id1),
                turn: Some(50),
                value: "30".to_string().into(),
            },
            MatchAttribute {
                name: "score".to_string(),
                bot_id: Some(bot_id1),
                turn: Some(50),
                value: "30".to_string().into(),
            },
        ];

        let m = Match::new(1234, vec![], attributes);

        let filter = MatchFilter::accept_all();
        assert!(filter.matches(&m));

        let filter = MatchFilter::from_str("match.initial_stones == 25").unwrap();
        assert!(filter.matches(&m));

        let filter = MatchFilter::from_str("match.initial_stones == 24").unwrap();
        assert!(!filter.matches(&m));

        let filter = MatchFilter::from_str("match.map_type == \"small\"").unwrap();
        assert!(filter.matches(&m));

        let filter = MatchFilter::from_str("match.stones_percentage == 0.75").unwrap();
        assert!(filter.matches(&m));

        let filter = MatchFilter::from_str(
            "match.stones_percentage > 0.7 AND match.stones_percentage < 0.8",
        )
        .unwrap();
        assert!(filter.matches(&m));

        let filter = MatchFilter::from_str(&format!("bot({bot_id1}).final_score >= 75")).unwrap();
        assert!(filter.matches(&m));

        let filter = MatchFilter::from_str(&format!(
            "bot({bot_id1}).final_score > bot({bot_id2}).final_score"
        ))
        .unwrap();
        assert!(filter.matches(&m));

        let filter = MatchFilter::from_str(&format!(
            "bot({bot_id1}).final_score < bot({bot_id2}).final_score"
        ))
        .unwrap();
        assert!(!filter.matches(&m));

        let filter = MatchFilter::from_str(&format!("bot({bot_id1})[50].score == 30")).unwrap();
        assert!(filter.matches(&m));

        let filter = MatchFilter::from_str(&format!("bot({bot_id1})[20].score == 30")).unwrap();
        assert!(!filter.matches(&m));

        let filter = MatchFilter::from_str(&format!("bot({bot_id3}).final_score == 75")).unwrap();
        assert!(!filter.matches(&m));

        let filter = MatchFilter::from_str("match.invalid_attr == 24").unwrap();
        assert!(!filter.matches(&m));
    }
}
