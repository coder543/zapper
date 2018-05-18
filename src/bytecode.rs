// during compilation, concatenate all string literals into a single "resource" string, and replace the literals with indices into the resource string.

use super::{Environment, FilterInput, Runner};
use ast::*;
use std::borrow::Cow;
use std::fmt::Debug;
use std::io::Write;
use tokenizer::Operator;

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
    buffer: Option<String>,
    stack: Option<Vec<f64>>,
    raw_text: String,
    instructions: Vec<Instr<NumEnum, StrEnum, FilterEnum>>,
}

macro_rules! pop {
    ($stack:ident) => {
        $stack.pop().unwrap_or_else(|| panic!("stack underflow!"))
    };
}

impl<
        'a,
        NumEnum: 'a + Copy + Debug,
        StrEnum: 'a + Copy + Debug + PartialEq,
        FilterEnum: 'a + Copy + Debug,
    > Bytecode<NumEnum, StrEnum, FilterEnum>
{
    #[allow(unused)]
    pub fn from_ast<Env: Environment<'a, NumEnum, StrEnum, FilterEnum>>(
        ast: Vec<Expr>,
        env: &Env,
    ) -> Result<Bytecode<NumEnum, StrEnum, FilterEnum>, String> {
        let mut ret_val = Bytecode {
            buffer: None,
            stack: None,
            raw_text: String::new(),
            instructions: vec![],
        };

        for tree in ast {
            ret_val.extend_with_tree(tree, env)?;
        }

        Ok(ret_val)
    }

    /// Renders a template using convenient internally-managed buffers, which requires a mutable reference to self.
    pub fn render(
        &mut self,
        runner: &Runner<NumEnum, StrEnum, FilterEnum>,
        output: &mut Write,
    ) -> Result<(), ::std::io::Error> {
        let mut stack = self.stack.take().unwrap_or_else(|| Vec::with_capacity(8));
        let mut buffer = self
            .buffer
            .take()
            .unwrap_or_else(|| String::with_capacity(8));

        let result = self.render_with(runner, output, &mut stack, &mut buffer);

        self.stack = Some(stack);
        self.buffer = Some(buffer);

        result
    }

    /// Renders a template using only externally provided buffers, allowing for parallelizing the render process by using
    /// buffers that are local to the current thread. This allows it to require only an immutable reference to self.
    pub fn render_with(
        &self,
        runner: &Runner<NumEnum, StrEnum, FilterEnum>,
        output: &mut Write,
        stack: &mut Vec<f64>,
        buffer: &mut String,
    ) -> Result<(), ::std::io::Error> {
        for instr in &self.instructions {
            match *instr {
                Instr::PushImm(val) => stack.push(val),
                Instr::PushNum(id) => stack.push(runner.num_var(id)),
                Instr::PrintReg => write!(output, "{}", pop!(stack))?,
                Instr::PrintRaw(start, end) => write!(output, "{}", &self.raw_text[start..end])?,
                Instr::PrintStr(id) => write!(output, "{}", runner.str_var(id))?,
                Instr::PrintNum(id) => write!(output, "{}", runner.num_var(id))?,
                Instr::Add => {
                    let right = pop!(stack);
                    let left = pop!(stack);
                    let result = left + right;
                    stack.push(result)
                }
                Instr::Sub => {
                    let right = pop!(stack);
                    let left = pop!(stack);
                    let result = left - right;
                    stack.push(result)
                }
                Instr::Mul => {
                    let right = pop!(stack);
                    let left = pop!(stack);
                    let result = left * right;
                    stack.push(result)
                }
                Instr::Div => {
                    let right = pop!(stack);
                    let left = pop!(stack);
                    let result = left / right;
                    stack.push(result)
                }
                Instr::CallReg(id, ref args) => {
                    write!(output, "{}", runner.filter_num(id, args, pop!(stack)))?
                }
                Instr::CallId(id, ref args, val_id) => {
                    buffer.clear();
                    runner.filter_id(id, args, val_id, &mut *buffer);
                    write!(output, "{}", buffer)?
                }
                Instr::CallStr(id, ref args, val_id) => {
                    let string = runner.str_var(val_id);
                    buffer.clear();
                    runner.filter_str(id, args, string, buffer);
                    write!(output, "{}", buffer)?
                }
                Instr::CallRegStr(id, ref args) => {
                    //CallRegStr could probably do without this string allocation
                    let string = pop!(stack).to_string();
                    buffer.clear();
                    runner.filter_str(id, args, Cow::from(string), buffer);
                    write!(output, "{}", buffer)?
                }
            }
        }

        Ok(())
    }

    fn extend_with_tree<Env: Environment<'a, NumEnum, StrEnum, FilterEnum>>(
        &mut self,
        tree: Expr,
        env: &Env,
    ) -> Result<(), String> {
        match tree {
            Expr::Raw(string) => {
                let start = self.raw_text.len();
                let end = start + string.len();
                self.raw_text.push_str(string);
                self.instructions.push(Instr::PrintRaw(start, end));
            }
            Expr::StringLiteral(string) => {
                let start = self.raw_text.len();
                let end = start + string.len();
                self.raw_text.push_str(&string);
                self.instructions.push(Instr::PrintRaw(start, end));
            }
            Expr::Identifier(id) => {
                if let Some(val) = Env::num_var(id) {
                    self.instructions.push(Instr::PrintNum(val));
                } else if let Some(val) = Env::str_var(id) {
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

    fn extend_with_numeric<Env: Environment<'a, NumEnum, StrEnum, FilterEnum>>(
        &mut self,
        numeric: Numeric,
        env: &Env,
    ) -> Result<(), String> {
        match numeric {
            Numeric::Raw(val) => {
                self.instructions.push(Instr::PushImm(val));
            }
            Numeric::Identifier(id) => {
                if let Some(val) = Env::num_var(id) {
                    self.instructions.push(Instr::PushNum(val));
                } else if let Some(_) = Env::str_var(id) {
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

    fn extend_with_filter<Env: Environment<'a, NumEnum, StrEnum, FilterEnum>>(
        &mut self,
        id: &str,
        expr: Expr,
        args: Vec<Literal>,
        env: &Env,
    ) -> Result<(), String> {
        if let Some((val, arg_count, input_type)) = Env::filter(id) {
            if arg_count != args.len() {
                return Err(format!(
                    "filter {} expected {} args, but {} were provided",
                    id,
                    arg_count,
                    args.len()
                ));
            }

            let args: Result<Vec<f64>, String> = args
                .into_iter()
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
                    if let Some(val_id) = Env::num_var(val_id) {
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
                    match Env::str_var(val_id) {
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
                    if let Some(val_id) = Env::str_var(val_id) {
                        self.instructions.push(Instr::CallStr(val, args?, val_id));
                    } else if let Some(val_id) = Env::num_var(val_id) {
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
        } else {
            return Err(format!("Unknown filter named {}", id));
        }

        Ok(())
    }
}
