extern crate zap;
extern crate handlebars;

#[macro_use]
extern crate criterion;
use criterion::Criterion;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use zap::{compile, Environment, FilterInput, Runner};
use handlebars::{to_json, Handlebars};

#[derive(Clone, Serialize)]
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

fn bench_zap(c: &mut Criterion) {
    let template =
        "{{provider}} {{provider_code}} {{id}} {{name}} {{age}} {{weight}}kg\n";
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
    let runner = runner();

    c.bench_function("zap", move |b| b.iter(|| {
        let mut output = Vec::new();
        for person in &group {
            bytecode
                .run_with(&runner, person, &mut buffer, &mut stack, &mut output)
                .unwrap();
        }
        output
    }));
}

fn bench_hbs(c: &mut Criterion) {
    use serde_json::value::Map;
    let template =
        "{{#each group as |p| ~}}{{provider}} {{provider_code}} {{p.id}} {{p.name}} {{p.age}} {{p.weight}}kg\n{{/each~}}";
    
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("table", template).unwrap();

    let mut group = vec![];
    for i in 0..100 {
        group.push(Person {
            id: 12 + i,
            name: "Bob".to_string(),
            age: 49,
            weight: 170.3 + i as f64,
        });
    }

    let mut data = Map::new();
    data.insert("provider".to_string(), to_json(&"apns".to_string()));
    data.insert("provider_code".to_string(), to_json(&"35".to_string()));
    c.bench_function("hbs", move |b| b.iter(|| {
        let mut data = data.clone();
        data.insert("group".to_string(), to_json(&group));
        handlebars.render("table", &data).unwrap()
    }));
}

pub fn benches() {
    use std::time::Duration;
    let mut criterion: Criterion = Criterion::default()
        .configure_from_args()
        .sample_size(200)
        .measurement_time(Duration::from_secs(40));
    bench_zap(&mut criterion);
    bench_hbs(&mut criterion);
}

criterion_main!(benches);