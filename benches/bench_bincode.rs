#![feature(test)]
extern crate test;
extern crate serde;

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
        let a = bincode::serialize(&a).unwrap();
        let b: A = bincode::deserialize(&a).unwrap();
    });
}