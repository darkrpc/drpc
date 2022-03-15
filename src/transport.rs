use crate::frame::Frame;

/// client Transport,server don't need
#[async_trait]
pub trait Transport {
    async fn transport(&self, frame: Frame) -> Frame;
}