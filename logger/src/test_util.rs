use crate::{Message, Pod};

#[derive(strum_macros::Display)]
pub enum TestMessage {
    A,
    B,
}

impl Message for TestMessage {
    fn process(&self) -> Pod {
        Pod::SystemServer
    }
}
