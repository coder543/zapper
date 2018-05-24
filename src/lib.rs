#[allow(unused_imports)]
#[cfg(feature = "derive")]
#[macro_use]
extern crate zapper_derive;
#[cfg(feature = "derive")]
pub use zapper_derive::*;

pub mod ast;
pub mod bytecode;
pub mod optimizer;
pub mod tokenizer;

use std::borrow::Cow;
use std::fmt::Debug;

pub use bytecode::Bytecode;

pub enum FilterInput<StrEnum> {
    Numeric,
    StrEnumId(Vec<StrEnum>),
    Stringified,
}

pub trait Environment<'a, NumEnum: 'a, StrEnum: 'a + Debug + PartialEq, FilterEnum: 'a> {
    fn num_constant(&self, &str) -> Option<f64>;
    fn str_constant(&'a self, &str) -> Option<Cow<'a, str>>;

    fn num_var(&str) -> Option<NumEnum>;
    fn str_var(&str) -> Option<StrEnum>;

    // returns a FilterEnum, the number of arguments, and the input data type
    fn filter(&str) -> Option<(FilterEnum, usize, FilterInput<StrEnum>)>;
}

#[allow(unused)]
pub trait Runner<NumEnum, StrEnum, FilterEnum> {
    fn num_var(&self, NumEnum) -> f64;
    fn str_var(&self, StrEnum) -> Cow<str>;

    fn filter_num(&self, FilterEnum, &[f64], f64) -> f64;

    // the fourth argument is a reusable buffer to reduce allocation
    fn filter_id(&self, FilterEnum, &[f64], StrEnum, &mut String);
    fn filter_str(&self, FilterEnum, &[f64], Cow<str>, &mut String);
}

pub fn compile<
    'a,
    NumEnum: 'a + Copy + Debug,
    StrEnum: 'a + Copy + Debug + PartialEq,
    FilterEnum: 'a + Copy + Debug,
    Env: Environment<'a, NumEnum, StrEnum, FilterEnum>,
>(
    source: &'a str,
    environment: &'a Env,
) -> Result<Bytecode<NumEnum, StrEnum, FilterEnum>, String> {
    let tokenizer = tokenizer::Tokenizer::new(source);
    let ast = ast::parse(tokenizer)?;
    // println!("ast: {:#?}\n", ast);
    let ast = optimizer::optimize(ast, environment);
    // println!("ast_opt: {:#?}\n", ast);
    Bytecode::from_ast(ast, environment)
}
