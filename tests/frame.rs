#[cfg(test)]
mod test {
    use drpc::frame::Frame;
    use std::io::Error;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};

    pub struct Mock {
        pub inner: Vec<u8>,
        pub pos: usize,
    }

    impl AsyncRead for Mock {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            buf.put_slice(&self.inner[self.pos..self.pos + buf.initialized().len()]);
            self.get_mut().pos += buf.filled().len();
            return Poll::Ready(Ok(()));
        }
    }

    impl AsyncWrite for Mock {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, Error>> {
            self.get_mut().inner.extend(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_frame() {
        let mut req = Frame::new();
        let body = "hello".as_bytes();
        println!("body={:?}", body);
        let _ = req.write_all(body).await;
        let data = req.finish(100);
        let mut mock = Mock {
            inner: data,
            pos: 0,
        };
        println!("data={:?}", mock.inner);
        let f = Frame::decode_from(&mut mock).await.unwrap();
        println!("id={},ok={},data={:?}", f.id, f.ok, f.data);
    }
}
