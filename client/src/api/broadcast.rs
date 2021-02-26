use async_channel::{Receiver, Sender};
use basws_client::Handle;

#[derive(Debug)]
pub struct BroadcastChannel<T> {
    data: Handle<Option<BroadcastChannelData<T>>>,
}

#[derive(Debug)]
struct BroadcastChannelData<T> {
    receivers: Vec<Sender<T>>,
}

impl<T> Default for BroadcastChannel<T> {
    fn default() -> Self {
        Self {
            data: Handle::new(None),
        }
    }
}

impl<T> Clone for BroadcastChannel<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl<T: Clone + 'static> BroadcastChannel<T> {
    pub async fn receiver(&self) -> Receiver<T> {
        let mut data = self.data.write().await;
        if data.is_none() {
            *data = Some(BroadcastChannelData {
                receivers: Vec::new(),
            });
        }
        let data = data.as_mut().unwrap();
        let (sender, receiver) = async_channel::unbounded();
        data.receivers.push(sender);
        receiver
    }

    pub async fn send(&self, value: T) -> Result<(), async_channel::SendError<T>> {
        let mut data = self.data.write().await;
        if let Some(channel) = data.as_mut() {
            let mut disconnected_indices = Vec::new();

            for (index, sender) in channel.receivers.iter().enumerate() {
                if let Err(async_channel::SendError(_)) = sender.send(value.clone()).await {
                    disconnected_indices.push(index);
                }
            }

            for index in disconnected_indices.into_iter().rev() {
                channel.receivers.remove(index);
            }

            Ok(())
        } else {
            Err(async_channel::SendError(value))
        }
    }
}
