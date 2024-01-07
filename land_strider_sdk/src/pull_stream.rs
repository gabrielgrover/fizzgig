use bytes::Bytes;
use futures::TryStreamExt;
use serde_json::Value;
use std::pin::Pin;
use std::task::Poll;
use tokio_stream::Stream;

#[derive(Debug, thiserror::Error)]
enum PullStreamErr {
    #[error("Failed to process stream item: {0}")]
    ItemProcess(String),
    #[error("Failed to parse stream data: {0}")]
    ItemParse(String),
}

pub type BytesStream =
    Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send>>> + Send>>;

pub struct PullStream {
    bytes_stream: BytesStream,
    buf: Vec<u8>,
    left_over_bytes: Option<Vec<u8>>,
}

impl PullStream {
    pub fn new(bytes_stream: BytesStream) -> Self {
        Self {
            bytes_stream,
            buf: vec![],
            left_over_bytes: None,
        }
    }
}

impl From<reqwest::Response> for PullStream {
    fn from(resp: reqwest::Response) -> Self {
        PullStream::new(Box::pin(
            resp.bytes_stream()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>),
        ))
    }
}

impl Stream for PullStream {
    type Item = Result<Value, Box<dyn std::error::Error>>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let left_over_bytes = self.left_over_bytes.take().unwrap_or(vec![]);

        if !left_over_bytes.is_empty() {
            let mut break_idx = 0;
            for (idx, byte) in left_over_bytes.iter().enumerate() {
                let received_new_line = byte == &b'\n';

                if received_new_line && !self.buf.is_empty() {
                    let slice = self.buf.as_slice();
                    let v: Value = serde_json::from_slice(slice)
                        .map_err(|e| PullStreamErr::ItemParse(e.to_string()))?;

                    self.buf.clear();
                    self.left_over_bytes = Some(
                        left_over_bytes
                            .iter()
                            .skip(break_idx + 1)
                            .map(|i| *i)
                            .collect(),
                    );

                    return Poll::Ready(Some(Ok(v)));
                } else if !received_new_line {
                    break_idx = idx;
                    self.buf.push(*byte);
                }
            }
        }

        loop {
            let byte_stream_poll_status = Pin::new(&mut self.bytes_stream).poll_next(cx);

            match byte_stream_poll_status {
                Poll::Ready(Some(Ok(bytes))) => {
                    let mut break_idx = 0;

                    for (idx, byte) in bytes.iter().enumerate() {
                        let received_new_line = byte == &b'\n';

                        if received_new_line && !self.buf.is_empty() {
                            let slice = self.buf.as_slice();
                            let v: Value = serde_json::from_slice(slice)
                                .map_err(|e| PullStreamErr::ItemParse(e.to_string()))?;

                            self.buf.clear();
                            self.left_over_bytes =
                                Some(bytes.iter().skip(break_idx + 1).map(|i| *i).collect());

                            return Poll::Ready(Some(Ok(v)));
                        } else if !received_new_line {
                            break_idx = idx;
                            self.buf.push(*byte);
                        }
                    }
                }

                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(Box::new(PullStreamErr::ItemProcess(
                        e.to_string(),
                    )))));
                }

                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }

                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}
