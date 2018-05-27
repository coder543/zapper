extern crate handlebars;

#[macro_use]
extern crate zapper;

#[macro_use]
extern crate criterion;
use criterion::Criterion;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use handlebars::{to_json, Handlebars};
use zapper::compile;

#[derive(Clone, ZapperRunner, Serialize)]
#[filter = "sqrt/0n"]
#[filter = "round/1n"]
#[filter = "toupper/0s"]
struct Person {
    id: u64,
    name: String,
    age: u32,
    weight: f64,
}

#[derive(ZapperEnv)]
#[runner = "Person"]
struct Provider {
    provider: String,
    provider_code: u32,
}

fn sqrt(_data: &Person, _args: &[f64], input: f64) -> f64 {
    input.sqrt()
}

fn round(_data: &Person, args: &[f64], input: f64) -> f64 {
    let digits = args[0];
    let factor = 10u32.pow(digits as u32) as f64;
    let value = (input * factor).round() as f64;
    value / factor
}

fn toupper(_data: &Person, _args: &[f64], input: &str, buffer: &mut String) {
    for c in input.as_bytes() {
        buffer.push(c.to_ascii_uppercase() as char)
    }
}

fn bench_zapper(c: &mut Criterion) {
    let template = "{{provider}} {{provider_code}} {{id}} {{name}} {{age}} {{weight}}kg\n";
    let env = Provider {
        provider: "apns".to_string(),
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

    c.bench_function("zapper", move |b| {
        b.iter(|| {
            let mut output = Vec::new();
            for person in &group {
                bytecode.render(person, &mut output).unwrap();
            }
            output
        })
    });
}

fn bench_zapper_par(c: &mut Criterion) {
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

    c.bench_function("zapper_par", move |b| {
        b.iter(|| {
            let mut output = Vec::new();
            bytecode.par_render(&group, &mut output).unwrap();
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
    bench_zapper(&mut criterion);
    bench_zapper_par(&mut criterion);
    bench_hbs(&mut criterion);
}

criterion_main!(benches);
