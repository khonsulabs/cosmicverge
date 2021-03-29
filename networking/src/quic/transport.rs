use quinn::TransportConfig;

/// TODO: docs
pub(super) fn transport() -> TransportConfig {
    let mut transport = TransportConfig::default();
    #[allow(clippy::expect_used)]
    let _ = transport
        // TODO: research if this is necessary, it improves privacy, but may hurt network providers?
        .allow_spin(false)
        // TODO: handle keep-alive and time-out
        // transport.keep_alive_interval(); // heartbeat to prevent time-out, only needs to be sent from one side
        // transport.max_idle_timeout(); // time before being dropped
        // this API has no support for sending unordered data
        .datagram_receive_buffer_size(None)
        // TODO: support more then a single bidi-stream per connection
        .max_concurrent_bidi_streams(1)
        .expect("can't be bigger then `VarInt`")
        // TODO: handle uni streams
        .max_concurrent_uni_streams(0)
        .expect("can't be bigger then `VarInt`")
        // TODO: handle credits
        // .stream_receive_window() // total bytes receive buffer for a stream: (maximum bytes allowed per stream) * (expected latency)
        // .receive_window() // total bytes receive buffer for all streams of a single peer: (maximum number of streams) * (stream receive window)
        // .send_window() // total bytes send buffer for all streams of a single peer
        // TODO: handle congestion, needs research
        // .congestion_controller_factory()
        ;
    transport
}
