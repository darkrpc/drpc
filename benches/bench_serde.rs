#![feature(test)]
extern crate test;
extern crate serde;
extern crate serde_json;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct A {
    pub id: i32,
    pub name: String,
}

#[bench]
fn bench_bincode(b: &mut test::Bencher) {
    b.iter(|| {
        let a = A {
            id: 0,
            name: "".to_string(),
        };
        let a = serde_json::to_string(&a).unwrap();
        let b: A = serde_json::from_str(&a).unwrap();
    });
}