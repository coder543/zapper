// during compilation, concatenate all string literals into a single "resource" string, and replace the literals with indices into the resource string.

use std::io::Write;
use std::fmt::Debug;
use tokenizer::Operator;
use ast::*;
use super::{Environment, FilterInput};

#[allow(unused)]
pub struct Runner<'a, Data: 'a, NumEnum, StrEnum, FilterEnum> {
    pub num_var: fn(&Data, &NumEnum) -> f64,
    pub str_var: fn(&'a Data, &StrEnum) -> &'a str,

    pub filter_num: fn(&Data, &FilterEnum, &[f64], f64) -> f64,
    // the fourth argument is an optionally reusable buffer to reduce allocation
    pub filter_id: fn(&Data, &FilterEnum, &[f64], &StrEnum, String) -> String,
    pub filter_str: fn(&Data, &FilterEnum, &[f64], &str, String) -> String,
}

#[allow(unused)]
#[derive(Debug)]
enum Instr<NumEnum, StrEnum, FilterEnum> {
    PrintRaw(usize, usize), //prints a range from the resource string
    PrintStr(StrEnum),
    PrintNum(NumEnum),
    PrintReg,

    PushImm(f64),
    PushNum(NumEnum),
    CallReg(FilterEnum, Vec<f64>),
    CallId(FilterEnum, Vec<f64>, StrEnum),
    CallStr(FilterEnum, Vec<f64>, StrEnum),
    CallRegStr(FilterEnum, Vec<f64>),
    Add,
    Sub,
    Mul,
    Div,
}

#[allow(unused)]
#[derive(Debug)]
pub struct Bytecode<NumEnum, StrEnum, FilterEnum> {
    raw_text: String,
    instructions: Vec<Instr<NumEnum, StrEnum, FilterEnum>>,
}

macro_rules! pop {
    ($stack: ident) => {
        $stack.pop().unwrap_or_else(|| panic!("stack underflow!"))
    };
}

impl<NumEnum: Debug, StrEnum: Debug + PartialEq, FilterEnum: Debug>
    Bytecode<NumEnum, StrEnum, FilterEnum>
{
    #[allow(unused)]
    pub fn from_ast<Data>(
        ast: Vec<Expr>,
        env: &Environment<Data, NumEnum, StrEnum, FilterEnum>,
    ) -> Result<Bytecode<NumEnum, StrEnum, FilterEnum>, String> {
        let mut ret_val = Bytecode {
            raw_text: String::new(),
            instructions: vec![],
        };

        for tree in ast {
            ret_val.extend_with_tree(tree, env)?;
        }

        Ok(ret_val)
    }

    pub fn run_with<'a, Data: 'a>(
        &self,
        runner: Runner<'a, Data, NumEnum, StrEnum, FilterEnum>,
        data: &'a Data,
        mut buffer: String,
        mut stack: Vec<f64>,
        output: &mut Write,
    ) -> Result<(String, Vec<f64>), ::std::io::Error> {
        for instr in &self.instructions {
            match instr {
                &Instr::PushImm(val) => stack.push(val),
                &Instr::PushNum(ref id) => stack.push((runner.num_var)(&data, id)),
                &Instr::PrintReg => write!(output, "{}", pop!(stack))?,
                &Instr::PrintRaw(start, end) => write!(output, "{}", &self.raw_text[start..end])?,
                &Instr::PrintStr(ref id) => write!(output, "{}", (runner.str_var)(data, id))?,
                &Instr::PrintNum(ref id) => write!(output, "{}", (runner.num_var)(data, id))?,
                &Instr::Add => {
                    let right = pop!(stack);
                    let left = pop!(stack);
                    let result = left + right;
                    stack.push(result)
                }
                &Instr::Sub => {
                    let right = pop!(stack);
                    let left = pop!(stack);
                    let result = left - right;
                    stack.push(result)
                }
                &Instr::Mul => {
                    let right = pop!(stack);
                    let left = pop!(stack);
                    let result = left * right;
                    stack.push(result)
                }
                &Instr::Div => {
                    let right = pop!(stack);
                    let left = pop!(stack);
                    let result = left / right;
                    stack.push(result)
                }
                &Instr::CallReg(ref id, ref args) => write!(
                    output,
                    "{}",
                    (runner.filter_num)(data, id, args, pop!(stack))
                )?,
                &Instr::CallId(ref id, ref args, ref val_id) => {
                    buffer.clear();
                    buffer = (runner.filter_id)(data, id, args, val_id, buffer);
                    write!(output, "{}", buffer)?
                }
                &Instr::CallStr(ref id, ref args, ref val_id) => {
                    let string = (runner.str_var)(data, val_id);
                    buffer.clear();
                    buffer = (runner.filter_str)(data, id, args, string, buffer);
                    write!(output, "{}", buffer)?
                }
                &Instr::CallRegStr(ref id, ref args) => {
                    let string = pop!(stack).to_string();
                    buffer.clear();
                    buffer = (runner.filter_str)(data, id, args, &string, buffer);
                    write!(output, "{}", buffer)?
                }
            }
        }

        Ok((buffer, stack))
    }

    fn extend_with_tree<Data>(
        &mut self,
        tree: Expr,
        env: &Environment<Data, NumEnum, StrEnum, FilterEnum>,
    ) -> Result<(), String> {
        match tree {
            Expr::Raw(string) | Expr::StringLiteral(string) => {
                let start = self.raw_text.len();
                let end = start + string.len();
                self.raw_text.push_str(string);
                self.instructions.push(Instr::PrintRaw(start, end));
            }
            Expr::Identifier(id) => {
                if let Some(val) = (env.num_var)(id) {
                    self.instructions.push(Instr::PrintNum(val));
                } else if let Some(val) = (env.str_var)(id) {
                    self.instructions.push(Instr::PrintStr(val));
                } else {
                    return Err(format!("Unknown identifier {:?}", id));
                }
            }
            Expr::Numeric(numeric) => {
                self.extend_with_numeric(numeric, env)?;
                self.instructions.push(Instr::PrintReg);
            }
            Expr::Filter(id, expr, args) => self.extend_with_filter(id, *expr, args, env)?,
        }

        Ok(())
    }

    fn extend_with_numeric<Data>(
        &mut self,
        numeric: Numeric,
        env: &Environment<Data, NumEnum, StrEnum, FilterEnum>,
    ) -> Result<(), String> {
        match numeric {
            Numeric::Raw(val) => {
                self.instructions.push(Instr::PushImm(val));
            }
            Numeric::Identifier(id) => {
                if let Some(val) = (env.num_var)(id) {
                    self.instructions.push(Instr::PushNum(val));
                } else if let Some(_) = (env.str_var)(id) {
                    return Err(format!("{:?} is a string, numeric value was expected!", id));
                } else {
                    return Err(format!("Unknown identifier {:?}", id));
                }
            }
            Numeric::Parentheses(expr) => self.extend_with_numeric(*expr, env)?,
            Numeric::Negate(expr) => {
                self.extend_with_numeric(*expr, env)?;
                self.instructions.push(Instr::PushImm(-1.0));
                self.instructions.push(Instr::Mul);
            }
            Numeric::Binary(op, left, right) => {
                self.extend_with_numeric(*left, env)?;
                self.extend_with_numeric(*right, env)?;
                match op {
                    Operator::Plus => self.instructions.push(Instr::Add),
                    Operator::Dash => self.instructions.push(Instr::Sub),
                    Operator::Slash => self.instructions.push(Instr::Div),
                    Operator::Asterisk => self.instructions.push(Instr::Mul),
                    _ => unreachable!(),
                }
            }
        }

        Ok(())
    }

    fn extend_with_filter<Data>(
        &mut self,
        id: &str,
        expr: Expr,
        args: Vec<Literal>,
        env: &Environment<Data, NumEnum, StrEnum, FilterEnum>,
    ) -> Result<(), String> {
        if let Some((val, arg_count, input_type)) = (env.filter)(id) {
            if arg_count != args.len() {
                return Err(format!(
                    "filter {} expected {} args, but {} were provided",
                    id,
                    arg_count,
                    args.len()
                ));
            }

            let args: Result<Vec<f64>, String> = args.into_iter()
                .map(|x| match x {
                    Literal::Number(val) => Ok(val),
                    Literal::StringLiteral(_) => {
                        Err("Filters can only be passed numeric arguments for now!".to_string())
                    }
                })
                .collect();

            match (input_type, expr) {
                (FilterInput::Numeric, Expr::Numeric(expr)) => {
                    self.extend_with_numeric(expr, env)?;
                    self.instructions.push(Instr::CallReg(val, args?));
                }
                (FilterInput::Stringified, Expr::Numeric(expr)) => {
                    self.extend_with_numeric(expr, env)?;
                    self.instructions.push(Instr::CallRegStr(val, args?));
                }
                (FilterInput::Numeric, Expr::Identifier(val_id)) => {
                    if let Some(val_id) = (env.num_var)(val_id) {
                        self.instructions.push(Instr::PushNum(val_id));
                        self.instructions.push(Instr::CallReg(val, args?));
                    } else {
                        return Err(format!(
                            "filter {} expected numeric input expression, found {:#?}",
                            id,
                            val_id
                        ));
                    }
                }
                (FilterInput::Numeric, expr) => {
                    return Err(format!(
                        "filter {} expected numeric input expression, found {:#?}",
                        id,
                        expr
                    ));
                }
                (FilterInput::StrEnumId(valid_ids), Expr::Identifier(val_id)) => {
                    match (env.str_var)(val_id) {
                        None => {
                            return Err(format!(
                                "filter {} expected one of these identifiers: {:#?}.\nUnknown identifier found: {:#?}",
                                id,
                                valid_ids,
                                val_id
                            ))
                        }
                        Some(val_id) => {
                            if !valid_ids.contains(&val_id) {
                                return Err(format!(
                                    "filter {} expected one of these identifiers: {:#?}.\nErroneous identifier found: {:#?}",
                                    id,
                                    valid_ids,
                                    val_id
                                ));
                            }

                            self.instructions.push(Instr::CallId(val, args?, val_id));
                        }
                    }
                }
                (FilterInput::Stringified, Expr::Identifier(val_id)) => {
                    if let Some(val_id) = (env.str_var)(val_id) {
                        self.instructions.push(Instr::CallStr(val, args?, val_id));
                    } else if let Some(val_id) = (env.num_var)(val_id) {
                        self.instructions.push(Instr::PushNum(val_id));
                        self.instructions.push(Instr::CallRegStr(val, args?));
                    } else {
                        return Err(format!(
                            "Unknown identifier {:?} used on filter {:?}",
                            val_id,
                            id
                        ));
                    }
                }
                (FilterInput::Stringified, Expr::Filter(filt_id, filt_expr, filt_args)) => {
                    self.extend_with_filter(filt_id, *filt_expr, filt_args, env)?;
                    return Err("Nested filters are not yet supported!".to_string());
                }
                (FilterInput::StrEnumId(valid_ids), expr) => {
                    return Err(format!(
                        "filter {} expected just an identifier as the input.\nValid identifiers for this filter were {:#?}.\nErroneous expression found: {:#?}",
                        id,
                        valid_ids,
                        expr
                    ));
                }
                (FilterInput::Stringified, Expr::StringLiteral(string)) => {
                    return Err(format!(
                        "filters cannot accept a string literal as input for now. String: {:?} used on filter {:?}",
                        string,
                        id
                    ));
                }
                (FilterInput::Stringified, Expr::Raw(_)) => {
                    unreachable!();
                }
            }
        }

        Ok(())
    }
}
