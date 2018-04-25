extern crate zap;

use zap::{compile, Environment, FilterInput, Runner};

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

fn environment<'a>(
    provider: Provider,
) -> Environment<'a, Provider, PersonNums, PersonStrs, PersonFilters> {
    Environment {
        constant_data: provider,
        num_constant: |data, name| match name {
            "provider_code" => Some(data.provider_code as f64),
            _ => None,
        },
        str_constant: |data, name| match name {
            "provider" => Some(&data.provider),
            _ => None,
        },
        num_var: |name| match name {
            "id" => Some(PersonNums::Id),
            "age" => Some(PersonNums::Age),
            "weight" => Some(PersonNums::Weight),
            _ => None,
        },
        str_var: |name| match name {
            "name" => Some(PersonStrs::Name),
            _ => None,
        },

        filter: |name| match name {
            "sqrt" => Some((PersonFilters::Sqrt, 0, FilterInput::Numeric)),
            "round" => Some((PersonFilters::Round, 1, FilterInput::Numeric)),
            "toupper" => Some((PersonFilters::ToUpper, 0, FilterInput::Stringified)),
            _ => None,
        },
    }
}

fn runner<'a>() -> Runner<Person, PersonNums, PersonStrs, PersonFilters> {
    Runner {
        num_var: |data, var| match var {
            PersonNums::Id => data.id as f64,
            PersonNums::Age => data.age as f64,
            PersonNums::Weight => data.weight as f64,
        },

        str_var: |data, var| match var {
            PersonStrs::Name => &data.name,
        },

        filter_num: |_data, filter, args, input| match filter {
            PersonFilters::Sqrt => input.sqrt(),
            PersonFilters::Round => {
                let digits = args[0];
                let factor = 10u32.pow(digits as u32) as f64;
                let value = input * factor;
                let value = value.round() as f64;
                value / factor
            }
            _ => unreachable!(),
        },

        filter_id: |_data, _filter, _args, _input_id, _buffer| unreachable!(),

        filter_str: |_data, filter, _args, input, buffer| match filter {
            PersonFilters::ToUpper => for c in input.as_bytes() {
                buffer.push(c.to_ascii_uppercase() as char)
            },
            _ => unreachable!(),
        },
    }
}

fn main() {
    let template =
        "{{provider}} {{provider_code + 4}} {{id}} {{name | toupper}} {{age | sqrt}} {{weight / 2.2 | round 2}}kg\n";
    let env = environment(Provider {
        provider: "apns".to_string(),
        provider_code: 31,
    });
    let bytecode = match compile(template, &env) {
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

    // reuse these allocations throughout the output process
    let mut buffer = String::with_capacity(8);
    let mut stack = Vec::with_capacity(8);
    let stdout = stdout();
    let mut stdout_lock = stdout.lock();
    let runner = runner();

    for person in group {
        bytecode
            .run_with(&runner, &person, &mut buffer, &mut stack, &mut stdout_lock)
            .unwrap();
    }
}
