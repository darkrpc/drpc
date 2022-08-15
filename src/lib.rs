extern crate serde;
extern crate tokio;
#[macro_use]
extern crate async_trait;
extern crate log;
extern crate rand;

pub mod codec;
pub mod stub;
pub mod client;
pub mod server;
pub mod frame;
pub mod balance;
pub mod balance_manager;

use std::sync::atomic::Ordering;
pub use balance_manager::*;


pub use dark_std::errors::Error;
pub use dark_std::errors::Result;

/// set_frame_len
pub fn set_frame_len(size: u64) {
    crate::frame::FRAME_MAX_LEN.store(size, Ordering::SeqCst);
}

pub fn get_frame_len() -> u64 {
    crate::frame::FRAME_MAX_LEN.load(Ordering::SeqCst)
}