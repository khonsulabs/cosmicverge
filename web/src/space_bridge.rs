// Create a flow of messages from the canvas to the app, and the app to the canvas
//   App -> Canvas: Start/Stop animation, eventually more stuff including socket updates, scene changes, etc
//   Canvas -> App: Click events (eventually)

use crossbeam::channel::{self, Receiver, Sender};
use once_cell::sync::OnceCell;

static COMMAND_CHANNEL: OnceCell<(Sender<BridgeCommand>, Receiver<BridgeCommand>)> = OnceCell::new();
static EVENT_CHANNEL: OnceCell<(Sender<BridgeEvent>, Receiver<BridgeEvent>)> = OnceCell::new();

pub enum BridgeCommand {
    PauseRendering,
    ResumeRendering,
}

pub enum BridgeEvent {}


pub fn emit_event(event: BridgeEvent) -> Result<(), channel::SendError<BridgeEvent>> {
    let sender = &event_channel().0;
    sender.send(event)
}

pub fn event_receiver() -> Receiver<BridgeEvent> {
    event_channel().1.clone()
}

pub fn emit_command(command: BridgeCommand) -> Result<(), channel::SendError<BridgeCommand>> {
    let sender = &command_channel().0;
    sender.send(command)
}

pub fn command_receiver() -> Receiver<BridgeCommand> {
    command_channel().1.clone()
}

fn event_channel() -> &'static (Sender<BridgeEvent>, Receiver<BridgeEvent>) {
    EVENT_CHANNEL.get_or_init(|| channel::unbounded())
}

fn command_channel() -> &'static (Sender<BridgeCommand>, Receiver<BridgeCommand>) {
    COMMAND_CHANNEL.get_or_init(|| channel::unbounded())
}