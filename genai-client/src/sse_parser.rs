/// SSE (Server-Sent Events) parsing utilities
///
/// This module provides generic utilities for parsing SSE streams from the Gemini API.
/// SSE format consists of lines starting with "data: " followed by JSON payloads.
use crate::errors::InternalError;
use async_stream::try_stream;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::str;

/// Parses an SSE byte stream into a stream of deserialized objects.
///
/// This function handles the low-level SSE protocol parsing:
/// - Buffers incoming bytes
/// - Splits on newlines
/// - Extracts "data: " prefixed lines
/// - Deserializes JSON payloads
///
/// # Type Parameters
///
/// * `T` - The type to deserialize each SSE data payload into
///
/// # Arguments
///
/// * `byte_stream` - An async stream of byte chunks from the HTTP response
///
/// # Returns
///
/// A stream that yields deserialized objects of type `T` or errors
///
/// # Example
///
/// ```ignore
/// let byte_stream = response.bytes_stream();
/// let parsed_stream = parse_sse_stream::<MyResponseType>(byte_stream);
///
/// while let Some(result) = parsed_stream.next().await {
///     let response = result?;
///     // Process response...
/// }
/// ```
pub fn parse_sse_stream<T>(
    byte_stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send,
) -> impl Stream<Item = Result<T, InternalError>> + Send
where
    T: DeserializeOwned + Send,
{
    try_stream! {
        futures_util::pin_mut!(byte_stream);
        let mut buffer = Vec::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk: Bytes = chunk_result?;
            buffer.extend_from_slice(&chunk);

            while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                let line = str::from_utf8(&line_bytes)?.trim_end_matches(|c| c == '\n' || c == '\r');

                if line.starts_with("data:") {
                    let json_data = line.strip_prefix("data:").unwrap_or("").trim_start();
                    if !json_data.is_empty() {
                        let parsed: T = serde_json::from_str(json_data)?;
                        yield parsed;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{pin_mut, stream};
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestMessage {
        text: String,
    }

    #[tokio::test]
    async fn test_parse_sse_stream_single_message() {
        // Simulate SSE stream with a single message
        let data = b"data: {\"text\":\"Hello\"}\n\n".to_vec();
        let byte_stream = stream::iter(vec![Ok(Bytes::from(data))]);

        let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
        pin_mut!(parsed_stream);

        let result = parsed_stream.next().await;
        assert!(result.is_some());

        let message = result.unwrap().unwrap();
        assert_eq!(message.text, "Hello");
    }

    #[tokio::test]
    async fn test_parse_sse_stream_multiple_messages() {
        // Simulate SSE stream with multiple messages
        let data = b"data: {\"text\":\"First\"}\n\ndata: {\"text\":\"Second\"}\n\n".to_vec();
        let byte_stream = stream::iter(vec![Ok(Bytes::from(data))]);

        let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
        pin_mut!(parsed_stream);

        let first = parsed_stream.next().await.unwrap().unwrap();
        assert_eq!(first.text, "First");

        let second = parsed_stream.next().await.unwrap().unwrap();
        assert_eq!(second.text, "Second");
    }

    #[tokio::test]
    async fn test_parse_sse_stream_chunked_data() {
        // Simulate chunked SSE stream where data arrives in pieces
        let chunk1 = b"data: {\"te".to_vec();
        let chunk2 = b"xt\":\"Hello\"}\n\n".to_vec();
        let byte_stream = stream::iter(vec![Ok(Bytes::from(chunk1)), Ok(Bytes::from(chunk2))]);

        let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
        pin_mut!(parsed_stream);

        let message = parsed_stream.next().await.unwrap().unwrap();
        assert_eq!(message.text, "Hello");
    }

    #[tokio::test]
    async fn test_parse_sse_stream_ignores_non_data_lines() {
        // SSE streams can have comments and other lines we should ignore
        let data = b": comment\ndata: {\"text\":\"Hello\"}\n\nevent: test\n\n".to_vec();
        let byte_stream = stream::iter(vec![Ok(Bytes::from(data))]);

        let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
        pin_mut!(parsed_stream);

        let message = parsed_stream.next().await.unwrap().unwrap();
        assert_eq!(message.text, "Hello");

        // Should have no more messages
        assert!(parsed_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_empty_data_line() {
        // Empty "data: " lines should be skipped
        let data = b"data: \ndata: {\"text\":\"Hello\"}\n\n".to_vec();
        let byte_stream = stream::iter(vec![Ok(Bytes::from(data))]);

        let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
        pin_mut!(parsed_stream);

        let message = parsed_stream.next().await.unwrap().unwrap();
        assert_eq!(message.text, "Hello");
    }

    #[tokio::test]
    async fn test_parse_sse_stream_invalid_json() {
        // Invalid JSON should return an error
        let data = b"data: {invalid json}\n\n".to_vec();
        let byte_stream = stream::iter(vec![Ok(Bytes::from(data))]);

        let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
        pin_mut!(parsed_stream);

        let result = parsed_stream.next().await.unwrap();
        assert!(result.is_err());
    }
}
