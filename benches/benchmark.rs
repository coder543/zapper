extern crate handlebars;
extern crate zap;

#[macro_use]
extern crate criterion;
use criterion::Criterion;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use handlebars::{to_json, Handlebars};
use zap::{compile, Environment, FilterInput, Runner};

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

impl<'a> Environment<'a, PersonNums, PersonStrs, PersonFilters> for Provider {
    fn num_constant(&self, name: &str) -> Option<f64> {
        match name {
            "provider_code" => Some(self.provider_code as f64),
            _ => None,
        }
    }

    fn str_constant(&self, name: &str) -> Option<&str> {
        match name {
            "provider" => Some(&self.provider),
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

    fn str_var(&self, var: PersonStrs) -> &str {
        match var {
            PersonStrs::Name => &self.name,
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

    fn filter_str(&self, filter: PersonFilters, _args: &[f64], input: &str, buffer: &mut String) {
        match filter {
            PersonFilters::ToUpper => for c in input.as_bytes() {
                buffer.push(c.to_ascii_uppercase() as char)
            },
            _ => unreachable!(),
        }
    }
}

fn bench_zap(c: &mut Criterion) {
    let template = "{{provider}} {{provider_code}} {{id}} {{name}} {{age}} {{weight}}kg\n";
    let env = Provider {
        provider: "apns".to_string(),
        provider_code: 31,
    };
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

    c.bench_function("zap", move |b| {
        b.iter(|| {
            let mut output = Vec::new();
            for person in &group {
                bytecode
                    .run_with(person, &mut buffer, &mut stack, &mut output)
                    .unwrap();
            }
            output
        })
    });
}

fn bench_hbs(c: &mut Criterion) {
    use serde_json::value::Map;
    let template = "{{#each group as |p| ~}}{{provider}} {{provider_code}} {{p.id}} {{p.name}} {{p.age}} {{p.weight}}kg\n{{/each~}}";

    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("table", template)
        .unwrap();

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
    c.bench_function("hbs", move |b| {
        b.iter(|| {
            let mut data = data.clone();
            data.insert("group".to_string(), to_json(&group));
            handlebars.render("table", &data).unwrap()
        })
    });
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
