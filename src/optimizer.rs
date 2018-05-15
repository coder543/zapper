use super::Environment;
use ast::*;
use std::fmt::Debug;
use tokenizer::Operator;

#[allow(unused)]
pub fn optimize<
    'a,
    NumEnum: 'a,
    StrEnum: 'a + Debug + PartialEq,
    FilterEnum: 'a,
    Env: Environment<'a, NumEnum, StrEnum, FilterEnum>,
>(
    ast: Vec<Expr<'a>>,
    env: &'a Env,
) -> Vec<Expr<'a>> {
    ast.into_iter()
        .map(|tree| optimize_tree(tree, env, 20))
        .collect()
}

pub fn optimize_tree<
    'a,
    NumEnum: 'a,
    StrEnum: 'a + Debug + PartialEq,
    FilterEnum: 'a,
    Env: Environment<'a, NumEnum, StrEnum, FilterEnum>,
>(
    tree: Expr<'a>,
    env: &'a Env,
    effort: u32,
) -> Expr<'a> {
    if effort == 0 {
        return tree;
    }
    let effort = effort - 1;
    match tree {
        Expr::Identifier(id) => {
            if let Some(val) = env.num_constant(id) {
                Expr::Numeric(Numeric::Raw(val))
            } else if let Some(val) = env.str_constant(id) {
                Expr::StringLiteral(val)
            } else {
                Expr::Identifier(id)
            }
        }
        Expr::Numeric(numeric) => Expr::Numeric(optimize_numeric(numeric, env, effort)),
        Expr::Filter(id, expr, args) => {
            let expr = optimize_tree(*expr, env, effort);
            Expr::Filter(id, Box::new(expr), args)
        }
        expr => expr,
    }
}

pub fn optimize_numeric<
    'a,
    NumEnum: 'a,
    StrEnum: 'a + Debug + PartialEq,
    FilterEnum: 'a,
    Env: Environment<'a, NumEnum, StrEnum, FilterEnum>,
>(
    numeric: Numeric<'a>,
    env: &'a Env,
    effort: u32,
) -> Numeric<'a> {
    if effort == 0 {
        return numeric;
    }
    let effort = effort - 1;
    match numeric {
        Numeric::Identifier(id) => {
            if let Some(val) = env.num_constant(id) {
                Numeric::Raw(val)
            } else {
                Numeric::Identifier(id)
            }
        }
        Numeric::Binary(op, left, right) => {
            let left = optimize_numeric(*left, env, effort);
            let right = optimize_numeric(*right, env, effort);
            match (op, left, right) {
                (Operator::Plus, Numeric::Raw(left), Numeric::Raw(right)) => {
                    Numeric::Raw(left + right)
                }
                (Operator::Dash, Numeric::Raw(left), Numeric::Raw(right)) => {
                    Numeric::Raw(left - right)
                }
                (Operator::Slash, Numeric::Raw(left), Numeric::Raw(right)) => {
                    Numeric::Raw(left / right)
                }
                (Operator::Asterisk, Numeric::Raw(left), Numeric::Raw(right)) => {
                    Numeric::Raw(left * right)
                }
                (op, left, right) => Numeric::Binary(op, Box::new(left), Box::new(right)),
            }
        }
        Numeric::Negate(expr) => {
            let expr = optimize_numeric(*expr, env, effort);
            match expr {
                Numeric::Raw(val) => Numeric::Raw(-val),
                expr => Numeric::Negate(Box::new(expr)),
            }
        }
        Numeric::Parentheses(expr) => {
            let expr = optimize_numeric(*expr, env, effort);
            match expr {
                Numeric::Raw(val) => Numeric::Raw(val),
                expr => Numeric::Parentheses(Box::new(expr)),
            }
        }
        Numeric::Raw(raw) => Numeric::Raw(raw),
    }
}
