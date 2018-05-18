extern crate zap;

use zap::{compile, Environment, FilterInput, Runner};

use std::borrow::Cow;
use std::io::stdout;

#[derive(Clone)]
struct Person {
    id: u64,
    name: String,
    age: u32,
    weight: f64,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum PersonNums {
    Id,
    Age,
    Weight,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum PersonStrs {
    Name,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum PersonFilters {
    Sqrt,
    ToUpper,
    Round,
}

struct Provider {
    provider: String,
    provider_code: u32,
}

impl<'a> Environment<'a, PersonNums, PersonStrs, PersonFilters> for Provider {
    fn num_constant(&self, name: &str) -> Option<f64> {
        match name {
            "provider_code" => Some(self.provider_code as f64),
            _ => None,
        }
    }

    fn str_constant(&self, name: &str) -> Option<Cow<str>> {
        match name {
            "provider" => Some(Cow::from(&*self.provider)),
            _ => None,
        }
    }

    fn num_var(name: &str) -> Option<PersonNums> {
        match name {
            "id" => Some(PersonNums::Id),
            "age" => Some(PersonNums::Age),
            "weight" => Some(PersonNums::Weight),
            _ => None,
        }
    }

    fn str_var(name: &str) -> Option<PersonStrs> {
        match name {
            "name" => Some(PersonStrs::Name),
            _ => None,
        }
    }

    fn filter(name: &str) -> Option<(PersonFilters, usize, FilterInput<PersonStrs>)> {
        match name {
            "sqrt" => Some((PersonFilters::Sqrt, 0, FilterInput::Numeric)),
            "round" => Some((PersonFilters::Round, 1, FilterInput::Numeric)),
            "toupper" => Some((PersonFilters::ToUpper, 0, FilterInput::Stringified)),
            _ => None,
        }
    }
}

impl Runner<PersonNums, PersonStrs, PersonFilters> for Person {
    fn num_var(&self, var: PersonNums) -> f64 {
        match var {
            PersonNums::Id => self.id as f64,
            PersonNums::Age => self.age as f64,
            PersonNums::Weight => self.weight as f64,
        }
    }

    fn str_var(&self, var: PersonStrs) -> Cow<str> {
        match var {
            PersonStrs::Name => self.name.as_str().into(),
        }
    }

    fn filter_num(&self, filter: PersonFilters, args: &[f64], input: f64) -> f64 {
        match filter {
            PersonFilters::Sqrt => input.sqrt(),
            PersonFilters::Round => {
                let digits = args[0];
                let factor = 10u32.pow(digits as u32) as f64;
                let value = input * factor;
                let value = value.round() as f64;
                value / factor
            }
            _ => unreachable!(),
        }
    }

    fn filter_id(
        &self,
        _filter: PersonFilters,
        _args: &[f64],
        _input_id: PersonStrs,
        _buffer: &mut String,
    ) {
        unreachable!()
    }

    fn filter_str(
        &self,
        filter: PersonFilters,
        _args: &[f64],
        input: Cow<str>,
        buffer: &mut String,
    ) {
        match filter {
            PersonFilters::ToUpper => {
                for c in input.as_bytes() {
                    buffer.push(c.to_ascii_uppercase() as char)
                }
            }
            _ => unreachable!(),
        }
    }
}

fn main() {
    let template = "{{provider}} {{provider_code + 4}} {{id}} {{name | toupper}} {{age | sqrt}} {{weight / 2.2 | round 2}}kg\n";

    let env = Provider {
        provider: "john doe".to_string(),
        provider_code: 31,
    };

    let mut bytecode = match compile(template, &env) {
        Ok(bc) => bc,
        Err(err) => {
            eprintln!("error compiling template: {}", err);
            return;
        }
    };

    // println!("bytecode: {:#?}", bytecode);

    // build up a group of 100 (similar) people
    let mut group = vec![];
    for i in 0..100 {
        group.push(Person {
            id: 12 + i,
            name: "Bob".to_string(),
            age: 49,
            weight: 170.3 + i as f64,
        });
    }

    let stdout = stdout();
    let mut stdout_lock = stdout.lock();

    for person in group {
        bytecode.render(&person, &mut stdout_lock).unwrap();
    }
}
