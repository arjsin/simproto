use bytes::BytesMut;

#[derive(Clone, Copy, Debug)]
pub enum TypeLabel {
    Request,
    Response,
    Ping,
    Pong,
}

impl From<TypeLabel> for u8 {
    fn from(ftype: TypeLabel) -> Self {
        match ftype {
            TypeLabel::Request => 0,
            TypeLabel::Response => 1,
            TypeLabel::Ping => 2,
            TypeLabel::Pong => 3,
        }
    }
}

impl From<u8> for TypeLabel {
    fn from(byte: u8) -> Self {
        match byte {
            0 => TypeLabel::Request,
            1 => TypeLabel::Response,
            2 => TypeLabel::Ping,
            3 => TypeLabel::Pong,
            _ => TypeLabel::Request,
        }
    }
}

pub struct Frame {
    t: TypeLabel,
    id: u64,
    payload: BytesMut,
}

impl Frame {
    pub fn new(t: TypeLabel, id: u64, payload: BytesMut) -> Frame {
        Frame { t, id, payload }
    }

    pub fn frame_type(&self) -> TypeLabel {
        self.t
    }

    pub fn id(&self) -> u64{
        self.id
    }

    pub fn message_len(&self) -> usize {
        self.payload.len()
    }

    pub fn into_message(self) -> BytesMut {
        self.payload
    }
}
