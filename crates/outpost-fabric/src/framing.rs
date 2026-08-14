//! Length-prefixed `postcard` framing over any async byte stream.
//!
//! Frame = `u32` little-endian length + that many bytes of postcard. QUIC
//! streams, tokio duplex pipes, and unix sockets all qualify; the protocol
//! never knows which it runs on.

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Refuse absurd frames before allocating. Snapshots are the largest real
/// payloads; 64 MiB leaves generous headroom — a bigger frame is a bug or
/// an attack, not a document.
pub const MAX_FRAME: u32 = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame of {0} bytes exceeds MAX_FRAME")]
    TooLarge(u32),
    #[error("codec: {0}")]
    Codec(#[from] postcard::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Serialize and write one length-prefixed frame, then flush.
pub async fn write_frame<T: Serialize>(
    writer: &mut (impl AsyncWrite + Unpin),
    message: &T,
) -> Result<(), FrameError> {
    let bytes = postcard::to_stdvec(message)?;
    let len = u32::try_from(bytes.len()).map_err(|_| FrameError::TooLarge(u32::MAX))?;
    if len > MAX_FRAME {
        return Err(FrameError::TooLarge(len));
    }
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

/// Read one length-prefixed frame and decode it.
pub async fn read_frame<T: DeserializeOwned>(
    reader: &mut (impl AsyncRead + Unpin),
) -> Result<T, FrameError> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_le_bytes(len_bytes);
    if len > MAX_FRAME {
        return Err(FrameError::TooLarge(len));
    }
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf).await?;
    Ok(postcard::from_bytes(&buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::SyncMessage;

    #[tokio::test]
    async fn roundtrip_over_duplex() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        let msg = SyncMessage::ResupplyResponse { notices: vec![] };
        write_frame(&mut a, &msg).await.unwrap();
        let got: SyncMessage = read_frame(&mut b).await.unwrap();
        assert_eq!(got, msg);
    }

    #[tokio::test]
    async fn oversized_frame_is_rejected_before_alloc() {
        let (mut a, mut b) = tokio::io::duplex(64);
        let oversized = (MAX_FRAME + 1).to_le_bytes();
        a.write_all(&oversized).await.unwrap();
        let got: Result<SyncMessage, _> = read_frame(&mut b).await;
        assert!(matches!(got, Err(FrameError::TooLarge(_))));
    }
}
