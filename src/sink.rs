use crate::frame::{Frame, ReqBuf};

#[derive(Clone, Debug)]
pub struct Sink {}


impl Sink {
    pub fn encode(&self, data: Vec<u8>) -> Frame {
        todo!()
    }

    pub fn decode(&self, frames: Vec<Frame>) -> Frame {
        todo!()
    }
}