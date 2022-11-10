#![feature(test)]
extern crate serde;
extern crate test;

use serde::{Deserialize, Serialize};

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
