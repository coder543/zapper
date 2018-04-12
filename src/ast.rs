pub enum Expr<'a> {
    Raw(&'a str),
    RExpr(Rexpr<'a>),
    Filter1(&'a str, Rexpr<'a>),
    Filter2(&'a str, Rexpr<'a>, Rexpr<'a>),
}

// Restricted Expression
pub enum Rexpr<'a> {
    Identifier(&'a str),
    StringLiteral(&'a str),
    Add(Numeric<'a>, Numeric<'a>),
    Sub(Numeric<'a>, Numeric<'a>),
    Mul(Numeric<'a>, Numeric<'a>),
    Div(Numeric<'a>, Numeric<'a>),
}

pub enum Numeric<'a> {
    Raw(f64),
    Identifier(&'a str),
    Expr(Box<Expr<'a>>),
}
