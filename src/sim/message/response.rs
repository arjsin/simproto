use bytes::{BufMut, Bytes, BytesMut};

#[derive(Clone, PartialEq, Debug)]
pub enum RpcResponse {
    Accepted(Bytes),
    TopicNotFound,
    InvalidResponse,
}

#[derive(Clone, PartialEq, Debug)]
pub enum SubscriptionResponse {
    Accepted(Bytes),
    TopicNotFound,
    AlreadySubscribed,
    Rejected(Bytes),
    InvalidResponse,
}

#[derive(Clone, PartialEq, Debug)]
pub enum UnsubscriptionResponse {
    Accepted(Bytes),
    TopicNotFound,
    NotSubscribed,
    InvalidResponse,
}

#[derive(Clone, PartialEq, Debug)]
pub enum NotificationResponse {
    Notified,
    TopicNotFound,
    NotSubscribed,
    InvalidResponse,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Response {
    Accepted(Bytes),
    TopicNotFound,
    AlreadySubscribed,
    Rejected(Bytes),
    NotSubscribed,
    Notified,
    InvalidRequest,
    InvalidResponse,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum ResponseType {
    Accepted,
    TopicNotFound,
    AlreadySubscribed,
    Rejected,
    NotSubscribed,
    Notified,
    InvalidRequest,
    InvalidResponse,
}

impl From<Response> for RpcResponse {
    fn from(r: Response) -> Self {
        match r {
            Response::Accepted(x) => RpcResponse::Accepted(x),
            Response::TopicNotFound => RpcResponse::TopicNotFound,
            _ => RpcResponse::InvalidResponse,
        }
    }
}

impl From<Response> for SubscriptionResponse {
    fn from(r: Response) -> Self {
        match r {
            Response::Accepted(x) => SubscriptionResponse::Accepted(x),
            Response::TopicNotFound => SubscriptionResponse::TopicNotFound,
            Response::AlreadySubscribed => SubscriptionResponse::AlreadySubscribed,
            Response::Rejected(x) => SubscriptionResponse::Rejected(x),
            _ => SubscriptionResponse::InvalidResponse,
        }
    }
}

impl From<Response> for UnsubscriptionResponse {
    fn from(r: Response) -> Self {
        match r {
            Response::Accepted(x) => UnsubscriptionResponse::Accepted(x),
            Response::TopicNotFound => UnsubscriptionResponse::TopicNotFound,
            Response::NotSubscribed => UnsubscriptionResponse::NotSubscribed,
            _ => UnsubscriptionResponse::InvalidResponse,
        }
    }
}

impl From<Response> for NotificationResponse {
    fn from(r: Response) -> Self {
        match r {
            Response::Notified => NotificationResponse::Notified,
            Response::TopicNotFound => NotificationResponse::TopicNotFound,
            Response::NotSubscribed => NotificationResponse::NotSubscribed,
            _ => NotificationResponse::InvalidResponse,
        }
    }
}

impl From<RpcResponse> for Response {
    fn from(r: RpcResponse) -> Self {
        match r {
            RpcResponse::Accepted(x) => Response::Accepted(x),
            RpcResponse::TopicNotFound => Response::TopicNotFound,
            RpcResponse::InvalidResponse => Response::InvalidResponse,
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
            SubscriptionResponse::InvalidResponse => Response::InvalidResponse,
        }
    }
}

impl From<UnsubscriptionResponse> for Response {
    fn from(r: UnsubscriptionResponse) -> Self {
        match r {
            UnsubscriptionResponse::Accepted(x) => Response::Accepted(x),
            UnsubscriptionResponse::TopicNotFound => Response::TopicNotFound,
            UnsubscriptionResponse::NotSubscribed => Response::NotSubscribed,
            UnsubscriptionResponse::InvalidResponse => Response::InvalidResponse,
        }
    }
}

impl From<NotificationResponse> for Response {
    fn from(r: NotificationResponse) -> Self {
        match r {
            NotificationResponse::Notified => Response::Notified,
            NotificationResponse::TopicNotFound => Response::TopicNotFound,
            NotificationResponse::NotSubscribed => Response::NotSubscribed,
            NotificationResponse::InvalidResponse => Response::InvalidResponse,
        }
    }
}

impl ResponseType {
    fn from(b: u8) -> Self {
        match b {
            0 => ResponseType::Accepted,
            1 => ResponseType::TopicNotFound,
            2 => ResponseType::AlreadySubscribed,
            3 => ResponseType::Rejected,
            4 => ResponseType::NotSubscribed,
            5 => ResponseType::Notified,
            6 => ResponseType::InvalidRequest,
            _ => ResponseType::InvalidResponse,
        }
    }
}

impl Response {
    pub fn from_bytes(mut b: Bytes) -> Self {
        let kind: ResponseType = ResponseType::from(b.split_to(1)[0]);
        match kind {
            ResponseType::Accepted => {
                let message = b;
                Response::Accepted(message)
            }
            ResponseType::TopicNotFound => Response::TopicNotFound,
            ResponseType::AlreadySubscribed => Response::AlreadySubscribed,
            ResponseType::Rejected => {
                let message = b;
                Response::Rejected(message)
            }
            ResponseType::NotSubscribed => Response::NotSubscribed,
            ResponseType::Notified => Response::Notified,
            ResponseType::InvalidRequest => Response::InvalidRequest,
            ResponseType::InvalidResponse => Response::InvalidResponse,
        }
    }

    pub fn write(&self, b: &mut BytesMut) {
        match self {
            Response::Accepted(x) => {
                b.reserve(x.len() + 1);
                b.put_u8(0);
                b.put(x);
            }
            Response::TopicNotFound => {
                b.reserve(1);
                b.put_u8(1)
            }
            Response::AlreadySubscribed => {
                b.reserve(1);
                b.put_u8(2)
            }
            Response::Rejected(x) => {
                b.reserve(x.len() + 1);
                b.put_u8(3);
                b.put(x);
            }
            Response::NotSubscribed => {
                b.reserve(1);
                b.put_u8(4)
            }
            Response::Notified => {
                b.reserve(1);
                b.put_u8(5)
            }
            Response::InvalidRequest => {
                b.reserve(1);
                b.put_u8(6)
            }
            Response::InvalidResponse => panic!("invalid response"),
        }
    }
}
