use bytes::{BufMut, ByteOrder, Bytes, BytesMut, LittleEndian};

#[derive(Clone, Copy)]
pub enum RequestType {
    Rpc,
    Subscription,
    Unsubscription,
    Notification,
}

pub struct Request {
    pub kind: RequestType,
    pub topic: Bytes,
    pub message: Bytes,
}

impl RequestType {
    fn from(b: u8) -> Option<RequestType> {
        match b {
            0 => Some(RequestType::Rpc),
            1 => Some(RequestType::Subscription),
            2 => Some(RequestType::Unsubscription),
            3 => Some(RequestType::Notification),
            _ => None,
        }
    }
}

impl From<RequestType> for u8 {
    fn from(k: RequestType) -> Self {
        match k {
            RequestType::Rpc => 0,
            RequestType::Subscription => 1,
            RequestType::Unsubscription => 2,
            RequestType::Notification => 3,
        }
    }
}

impl Request {
    pub fn new(k: RequestType, topic: Bytes, message: Bytes) -> Request {
        debug_assert!(topic.len() <= (u16::max_value() as usize));
        Request {
            kind: k,
            topic,
            message,
        }
    }

    pub fn from_bytes(mut b: Bytes) -> Option<Request> {
        let kind: RequestType = RequestType::from(b.split_to(1)[0])?;
        let topic_len = LittleEndian::read_u16(&b.split_to(2));
        if (topic_len as usize) < b.len() {
            return None;
        }
        let topic = b.split_to(topic_len as usize);
        let message = b;
        Some(Request {
            kind,
            topic,
            message,
        })
    }

    pub fn write(&self, b: &mut BytesMut) {
        b.reserve(3 + self.topic.len() + self.message.len());
        b.put_u8(self.kind.into());
        b.put_u16_le(self.topic.len() as u16);
        b.extend_from_slice(&self.topic);
        b.extend_from_slice(&self.message);
    }
}
