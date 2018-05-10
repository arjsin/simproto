use super::frame::{Frame, TypeLabel};
use bytes::{BufMut, ByteOrder, BytesMut, LittleEndian};
use std::{io, mem};
use tokio_io::codec::{Decoder, Encoder};

pub struct Codec;

impl Decoder for Codec {
    type Item = Frame;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        const HEADER_LEN: usize = 1 + 2 * mem::size_of::<u64>();
        if buf.len() >= HEADER_LEN {
            let message_len = LittleEndian::read_u64(&buf[1..9]);
            if buf.len() >= message_len as usize + HEADER_LEN {
                let message_type = TypeLabel::from(buf[0]);
                let id = LittleEndian::read_u64(&buf[9..17]);
                buf.split_to(HEADER_LEN);
                let payload = buf.split_to(message_len as usize);
                Ok(Some(Frame::new(message_type, id, payload.freeze())))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder for Codec {
    type Item = Frame;
    type Error = io::Error;

    fn encode(&mut self, frame: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        let (t, id, payload) = frame.into();
        let len = payload.len() as usize + 5;
        buf.reserve(len);
        buf.put_u8(t.into());
        buf.put_u64_le(payload.len() as u64);
        buf.put_u64_le(id);
        buf.extend_from_slice(&payload);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::Bytes;

    fn sample_frame() -> Frame {
        let buf = [1u8, 2, 3, 4];
        let payload = Bytes::from(&buf[..]);
        Frame::new(TypeLabel::Response, 12, payload)
    }
    const ENCODED: [u8; 21] = [
        1, 4, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4,
    ];
    #[test]
    fn encode() {
        let frame = sample_frame();
        let mut encoded_bytesmut = BytesMut::new();
        let _ = Codec.encode(frame, &mut encoded_bytesmut).unwrap();
        assert_eq!(&encoded_bytesmut[..], &ENCODED);
    }
    #[test]
    fn decode_encode() {
        let mut encoded_bytesmut = BytesMut::with_capacity(ENCODED.len());
        encoded_bytesmut.put_slice(&ENCODED);
        let decoded = Codec.decode(&mut encoded_bytesmut).unwrap().unwrap();
        let mut encoded_again = BytesMut::new();
        let _ = Codec.encode(decoded, &mut encoded_again).unwrap();
        assert_eq!(&encoded_again[..], ENCODED);
    }
}
