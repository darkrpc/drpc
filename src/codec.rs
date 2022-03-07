use std::any::Any;
use std::ptr::NonNull;
use mco::err;
use mco::std::errors::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug)]
pub enum Codecs {
    BinCodec(BinCodec),
    JsonCodec(JsonCodec),
}

impl Default for Codecs {
    fn default() -> Self {
        Self::BinCodec(BinCodec {})
    }
}

pub trait Codec {
    fn encode<T: Serialize>(&self, arg: T) -> Result<Vec<u8>, Error>;
    fn decode<T: DeserializeOwned>(&self, arg: &[u8]) -> Result<T, Error>;
}

#[derive(Debug)]
pub struct JsonCodec {}

impl Codec for JsonCodec {
    fn encode<T: Serialize>(&self, arg: T) -> Result<Vec<u8>, Error> {
        match serde_json::to_vec(&arg) {
            Ok(ok) => { Ok(ok) }
            Err(e) => { Err(err!("{}",e)) }
        }
    }

    fn decode<T: DeserializeOwned>(&self, arg: &[u8]) -> Result<T, Error> {
        match serde_json::from_slice(arg) {
            Ok(v) => {
                Ok(v)
            }
            Err(e) => {
                Err(err!("{}",e))
            }
        }
    }
}

#[derive(Debug)]
pub struct BinCodec {}

impl Codec for BinCodec {
    fn encode<T: Serialize>(&self, arg: T) -> Result<Vec<u8>, Error> {
        match bincode::serialize(&arg) {
            Ok(ok) => { Ok(ok) }
            Err(e) => { Err(err!("{}",e)) }
        }
    }

    fn decode<T: DeserializeOwned>(&self, arg: &[u8]) -> Result<T, Error> {
        match bincode::deserialize(arg) {
            Ok(ok) => { Ok(ok) }
            Err(e) => { Err(err!("{}",e)) }
        }
    }
}

impl Codec for Codecs {
    fn encode<T: Serialize>(&self, arg: T) -> Result<Vec<u8>, Error> {
        match self {
            Codecs::BinCodec(s) => { s.encode(arg) }
            Codecs::JsonCodec(s) => { s.encode(arg) }
        }
    }

    fn decode<T: DeserializeOwned>(&self, arg: &[u8]) -> Result<T, Error> {
        match self {
            Codecs::BinCodec(s) => { s.decode(arg) }
            Codecs::JsonCodec(s) => { s.decode(arg) }
        }
    }
}