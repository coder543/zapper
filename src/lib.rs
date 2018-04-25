pub mod tokenizer;
pub mod ast;
pub mod optimizer;
pub mod bytecode;

use std::fmt::Debug;

pub use bytecode::Bytecode;
pub use bytecode::Runner;

pub enum FilterInput<StrEnum> {
    Numeric,
    StrEnumId(Vec<StrEnum>),
    Stringified,
}

#[allow(unused)]
pub struct Environment<'a, Data: 'a, NumEnum: 'a, StrEnum: 'a + Debug + PartialEq, FilterEnum: 'a> {
    pub constant_data: Data,
    pub num_constant: fn(&Data, &str) -> Option<f64>,
    pub str_constant: fn(&'a Data, &str) -> Option<&'a str>,

    pub num_var: fn(&str) -> Option<NumEnum>,
    pub str_var: fn(&str) -> Option<StrEnum>,

    // returns a FilterEnum, the number of arguments, and the input data type
    pub filter: fn(&str) -> Option<(FilterEnum, usize, FilterInput<StrEnum>)>,
}

pub fn compile<
    'a,
    Data,
    NumEnum: Copy + Debug,
    StrEnum: Copy + Debug + PartialEq,
    FilterEnum: Copy + Debug,
>(
    source: &'a str,
    environment: &'a Environment<'a, Data, NumEnum, StrEnum, FilterEnum>,
) -> Result<Bytecode<NumEnum, StrEnum, FilterEnum>, String> {
    let tokenizer = tokenizer::Tokenizer::new(source);
    let ast = ast::parse(tokenizer)?;
    // println!("ast: {:#?}\n", ast);
    let ast = optimizer::optimize(ast, environment);
    // println!("ast_opt: {:#?}\n", ast);
    Bytecode::from_ast(ast, environment)
}
