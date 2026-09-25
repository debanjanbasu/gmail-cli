//! Streaming MIME assembly and media-upload send/import operations.
//!
//! Outgoing messages are assembled incrementally: attachments are read
//! chunk-wise from disk and base64-encoded on the fly, so peak memory stays
//! proportional to a single chunk rather than the whole message. Uploads go
//! through Gmail's media-upload endpoints with raw `message/rfc822` bodies.

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};

use base64::Engine;
use bytes::Bytes;
use futures::stream::{Stream, unfold};
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::core::error::{GrrError, Result};
use crate::gmail::models::*;

const CHUNK: usize = 192 * 1024; // multiple of 3 for clean base64 framing

/// An attachment referenced by path, streamed into the outgoing MIME message.
pub struct StreamAttachment {
    pub path: PathBuf,
    pub filename: String,
    pub mime_type: String,
}

/// Incrementally reads one attachment file and yields padded-correct STANDARD
/// base64 frames. Full chunks are 3-byte aligned (CHUNK is a multiple of 3)
/// so their frames concatenate cleanly; only the final partial chunk carries padding.
struct PartEncoder {
    file: File,
    read_buf: Vec<u8>,
    encode_buf: Vec<u8>,
    finished: bool,
}

impl PartEncoder {
    async fn open(path: &Path) -> io::Result<Self> {
        Ok(Self {
            file: File::open(path).await?,
            read_buf: vec![0_u8; CHUNK],
            encode_buf: vec![0_u8; CHUNK.div_ceil(3) * 4],
            finished: false,
        })
    }

    async fn next_frame(&mut self) -> io::Result<Option<Bytes>> {
        if self.finished {
            return Ok(None);
        }
        let mut filled = 0usize;
        let mut eof = false;
        while filled < self.read_buf.len() {
            let n = self.file.read(&mut self.read_buf[filled..]).await?;
            if n == 0 {
                eof = true;
                break;
            }
            filled += n;
        }
        if filled == 0 {
            self.finished = true;
            return Ok(None);
        }
        let written = base64::engine::general_purpose::STANDARD
            .encode_slice(&self.read_buf[..filled], &mut self.encode_buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        if eof {
            self.finished = true;
        }
        Ok(Some(Bytes::copy_from_slice(&self.encode_buf[..written])))
    }
}

/// Unfold state: pending literal frames, remaining attachments, the encoder
/// for the attachment currently being streamed, and whether the closing
/// boundary has been queued.
struct MimeAssembler {
    frames: VecDeque<Bytes>,
    attachments: std::vec::IntoIter<StreamAttachment>,
    encoder: Option<PartEncoder>,
    trailer_queued: bool,
    boundary: String,
}

fn generate_boundary() -> String {
    format!(
        "----GRR_boundary_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis()),
        rand::random::<u32>()
    )
}

fn part_header_frame(attachment: &StreamAttachment, boundary: &str) -> Bytes {
    Bytes::from(format!(
        "\r\n\r\n--{boundary}\r\nContent-Type: {}\r\n\
         Content-Transfer-Encoding: base64\r\n\
         Content-Disposition: attachment; filename=\"{}\"\r\n\r\n",
        attachment.mime_type, attachment.filename
    ))
}

fn trailer_frame(boundary: &str) -> Bytes {
    Bytes::from(format!("\r\n\r\n--{boundary}--\r\n"))
}

/// Assemble a full RFC822 multipart message as a byte stream.
/// Files are read chunk-wise and base64-encoded incrementally.
///
/// The returned stream owns all message data (header strings are copied into
/// the initial frame eagerly), so precise-capture keeps it `'static` even
/// though the inputs are borrowed.
pub fn mime_message_stream(
    to: &str,
    subject: &str,
    plain_body: &str,
    attachments: Vec<StreamAttachment>,
    thread_id: Option<&str>,
) -> impl Stream<Item = io::Result<Bytes>> + use<> {
    let _ = thread_id;
    let boundary = generate_boundary();
    let mut frames = VecDeque::new();
    frames.push_back(Bytes::from(format!(
        "To: {to}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\r\n\
         --{boundary}\r\nContent-Type: text/plain; charset=\"UTF-8\"\r\n\r\n{plain_body}"
    )));

    unfold(
        MimeAssembler {
            frames,
            attachments: attachments.into_iter(),
            encoder: None,
            trailer_queued: false,
            boundary,
        },
        |mut state| async move {
            loop {
                if let Some(frame) = state.frames.pop_front() {
                    return Some((Ok(frame), state));
                }

                if let Some(mut encoder) = state.encoder.take() {
                    match encoder.next_frame().await {
                        Ok(Some(bytes)) => {
                            state.encoder = Some(encoder);
                            return Some((Ok(bytes), state));
                        }
                        Ok(None) => {}
                        Err(err) => return Some((Err(err), state)),
                    }
                }

                match state.attachments.next() {
                    Some(attachment) => match PartEncoder::open(&attachment.path).await {
                        Ok(encoder) => {
                            state
                                .frames
                                .push_back(part_header_frame(&attachment, &state.boundary));
                            state.encoder = Some(encoder);
                        }
                        Err(err) => return Some((Err(err), state)),
                    },
                    None => {
                        if state.trailer_queued {
                            return None;
                        }
                        state.trailer_queued = true;
                        let frame = trailer_frame(&state.boundary);
                        state.frames.push_back(frame);
                    }
                }
            }
        },
    )
}

impl super::GmailClient {
    /// Build a media-upload endpoint URL from the upload base.
    fn upload_url(&self, path: &str) -> Result<url::Url> {
        self.upload_base_url
            .join(path)
            .map_err(|e| GrrError::Config(format!("Invalid upload URL: {}", e)))
    }

    /// Send a fully-assembled RFC822 byte stream via the media-upload endpoint.
    ///
    /// The body is streamed straight from `stream`; nothing is buffered whole.
    pub async fn send_mime_stream<S>(&self, stream: S, thread_id: Option<&str>) -> Result<Message>
    where
        S: Stream<Item = io::Result<Bytes>> + Send + 'static,
    {
        let _ = thread_id;
        let mut url = self.upload_url("users/me/messages/send")?;
        url.query_pairs_mut().append_pair("uploadType", "media");
        let response = self
            .core
            .execute(
                self.core
                    .post(url)
                    .header(CONTENT_TYPE, HeaderValue::from_static("message/rfc822"))
                    .body(reqwest::Body::wrap_stream(stream)),
            )
            .await?;
        Ok(response.json().await?)
    }

    /// Import an RFC822 byte stream via the media-upload endpoint.
    pub async fn import_stream<S>(&self, stream: S, deleted: bool) -> Result<Message>
    where
        S: Stream<Item = io::Result<Bytes>> + Send + 'static,
    {
        let mut url = self.upload_url("users/me/messages/import")?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("uploadType", "media");
            q.append_pair("internalDateSource", "dateHeader");
            q.append_pair("deleted", &deleted.to_string());
        }
        let response = self
            .core
            .execute(
                self.core
                    .post(url)
                    .header(CONTENT_TYPE, HeaderValue::from_static("message/rfc822"))
                    .body(reqwest::Body::wrap_stream(stream)),
            )
            .await?;
        Ok(response.json().await?)
    }
}
