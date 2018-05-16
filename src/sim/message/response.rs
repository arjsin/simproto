use bytes::{Bytes, BytesMut, BufMut};

pub enum RpcResponse {
    Accepted(Bytes),
    TopicNotFound,
}

pub enum SubscriptionResponse {
    Accepted(Bytes),
    TopicNotFound,
    AlreadySubscribed,
    Rejected(Bytes),
}

pub enum UnsubscriptionResponse {
    Accepted(Bytes),
    TopicNotFound,
    NotSubscribed,
}

pub enum NotificationResponse {
    Notified,
    TopicNotFound,
    NotSubscribed,
}

pub enum Response {
    Accepted(Bytes),
    TopicNotFound,
    AlreadySubscribed,
    Rejected(Bytes),
    NotSubscribed,
    Notified,
}

enum ResponseType {
    Accepted,
    TopicNotFound,
    AlreadySubscribed,
    Rejected,
    NotSubscribed,
    Notified,
}

impl From<RpcResponse> for Response {
    fn from(r: RpcResponse) -> Self {
        match r {
            RpcResponse::Accepted(x) => Response::Accepted(x),
            RpcResponse::TopicNotFound => Response::TopicNotFound,
        }
    }
}

impl From<SubscriptionResponse> for Response {
    fn from(r: SubscriptionResponse) -> Self {
        match r {
            SubscriptionResponse::Accepted(x) => Response::Accepted(x),
            SubscriptionResponse::TopicNotFound => Response::TopicNotFound,
            SubscriptionResponse::Rejected(x) => Response::Rejected(x),
            SubscriptionResponse::AlreadySubscribed => Response::AlreadySubscribed,
        }
    }
}

impl From<UnsubscriptionResponse> for Response {
    fn from(r: UnsubscriptionResponse) -> Self {
        match r {
            UnsubscriptionResponse::Accepted(x) => Response::Accepted(x),
            UnsubscriptionResponse::TopicNotFound => Response::TopicNotFound,
            UnsubscriptionResponse::NotSubscribed => Response::NotSubscribed,
        }
    }
}

impl From<NotificationResponse> for Response {
    fn from(r: NotificationResponse) -> Self {
        match r {
            NotificationResponse::Notified => Response::Notified,
            NotificationResponse::TopicNotFound => Response::TopicNotFound,
            NotificationResponse::NotSubscribed => Response::NotSubscribed,
        }
    }
}

impl ResponseType {
    fn from(b: u8) -> Option<Self> {
        match b {
            0 => Some(ResponseType::Accepted),
            1 => Some(ResponseType::TopicNotFound),
            2 => Some(ResponseType::AlreadySubscribed),
            3 => Some(ResponseType::Rejected),
            4 => Some(ResponseType::NotSubscribed),
            5 => Some(ResponseType::Notified),
            _ => None,
        }
    }
}

impl Response {
    fn from_bytes(mut b: Bytes) -> Option<Self> {
        let kind: ResponseType = ResponseType::from(b.split_to(1)[0])?;
        match kind {
            ResponseType::Accepted => {
                let message = b;
                Some(Response::Accepted(message))
            }
            ResponseType::TopicNotFound => Some(Response::TopicNotFound),
            ResponseType::AlreadySubscribed => Some(Response::AlreadySubscribed),
            ResponseType::Rejected => {
                let message = b;
                Some(Response::Rejected(message))
            }
            ResponseType::NotSubscribed => Some(Response::NotSubscribed),
            ResponseType::Notified => Some(Response::Notified),
        }
    }

    fn write(&self, mut b: BytesMut) {
        match self {
            Response::Accepted(x) => {
                b.put_u8(0);
                b.extend_from_slice(x);
            },
            Response::TopicNotFound => b.put_u8(1),
            Response::AlreadySubscribed => b.put_u8(2),
            Response::Rejected(x) => {
                b.put_u8(3);
                b.extend_from_slice(x);
            },
            Response::NotSubscribed => b.put_u8(4),
            Response::Notified => b.put_u8(5),
        }
    }
}
