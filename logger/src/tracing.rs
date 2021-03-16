use std::{fmt::Display, sync::Arc};

use flume::Sender;
use tracing::{
    field::{Field, Visit},
    span, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::{Level, Log, Message, Pod};

pub struct Adapter {
    pod: Pod,
    sender: Sender<Arc<Log>>,
}

impl Adapter {
    #[must_use]
    pub fn new(pod: Pod, sender: Sender<Arc<Log>>) -> Self {
        Self { pod, sender }
    }
}

struct TracingMessage {
    pod: Pod,
    message: String,
}

impl Message for TracingMessage {
    fn process(&self) -> Pod {
        self.pod
    }
}

impl Display for TracingMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl<S: Subscriber + for<'lookup> LookupSpan<'lookup>> Layer<S> for Adapter {
    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(&id) {
            let level = match *span.metadata().level() {
                tracing::Level::DEBUG => Level::Debug,
                tracing::Level::INFO => Level::Info,
                tracing::Level::TRACE => Level::Trace,
                tracing::Level::ERROR => Level::Error,
                tracing::Level::WARN => Level::Warning,
            };

            let message = span.metadata().name().to_string();
            let log = Log::new(
                level,
                TracingMessage {
                    pod: self.pod,
                    message,
                },
            );

            // TODO add data from tracing

            self.sender.try_send(Arc::new(log)).unwrap();
        }
    }
}

struct LogConverter {
    log: Log,
}

impl Visit for LogConverter {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.log.add(field.name(), format!("{:?}", value)).unwrap();
    }
}
