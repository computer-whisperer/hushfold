//! The sound server's way: the game as a client of PulseAudio's protocol, which
//! PipeWire speaks as well. A stream opened on no sink in particular is the
//! server's to place, and it moves it when the system's output changes.

use super::{Device, Feed, Shared, Stream, lock};
use crate::audio::dsp::SR;
use crate::audio::engine::BLOCK;
use pulseaudio::protocol::stream::{BufferAttr, StreamFlags};
use pulseaudio::protocol::{self, Prop, SampleFormat, SampleSpec};
use pulseaudio::{AsPlaybackSource, Client, PlaybackStream};
use std::ffi::CString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

const NAME: &std::ffi::CStr = c"Hushfold Library";

/// Bytes in a frame: two channels of 32-bit floats.
const FRAME: u32 = 8;

/// Frames the server is asked to keep queued, end to end (46 ms), and the fewest
/// it asks for at a time.
const QUEUED: u32 = 2048;
const LEAST: u32 = 512;

pub struct Server {
    client: Client,
    /// A stream on this connection wasn't deleted, so its channel may yet be
    /// deleted again, under a stream given it after: connect afresh.
    spoiled: Arc<AtomicBool>,
}

impl Server {
    pub fn connect() -> Result<Server, String> {
        let client = Client::from_env(NAME).map_err(|e| e.to_string())?;
        let spoiled = Arc::default();
        Ok(Server { client, spoiled })
    }

    pub fn spoiled(&self) -> bool {
        self.spoiled.load(Ordering::Relaxed)
    }

    pub fn list(&self) -> (Vec<Device>, Option<String>) {
        let text = |s: &CString| s.to_string_lossy().into_owned();
        let sinks = pollster::block_on(self.client.list_sinks()).unwrap_or_default();
        let devices: Vec<Device> = (sinks.iter())
            .map(|s| {
                let id = text(&s.name);
                let name = s.description.as_ref().map_or(id.clone(), text);
                Device { id, name }
            })
            .collect();
        let info = pollster::block_on(self.client.server_info()).ok();
        let system = info.and_then(|i| i.default_sink_name).map(|n| text(&n));
        let system = system.and_then(|id| devices.iter().find(|d| d.id == id).map(|d| d.name.clone()));
        (devices, system)
    }

    pub fn has(&self, id: &str) -> bool {
        let Ok(name) = CString::new(id) else {
            return false;
        };
        pollster::block_on(self.client.lookup_sink_by_name(name)).is_ok()
    }

    /// A stream on the sink named `id`, or (None) on whichever the server places it
    /// (or `PULSE_SINK`, as libpulse's clients honour it).
    pub fn open(&self, id: Option<&str>, feed: Shared<Feed>) -> Result<Box<dyn Stream>, String> {
        // PipeWire places a stream for a sink that isn't there on the system's output,
        // as if it were there.
        if id.is_some_and(|id| !self.has(id)) {
            return Err("not there".into());
        }
        let env = std::env::var("PULSE_SINK").ok().filter(|s| !s.is_empty());
        let sink = id.map(str::to_owned).or(env);
        let sink_name = sink.map(CString::new).transpose().map_err(|e| e.to_string())?;
        let mut props = protocol::Props::new();
        props.set(Prop::MediaName, NAME);
        props.set(Prop::MediaRole, c"game");
        props.set(Prop::ApplicationId, c"hushfold");
        props.set(Prop::ApplicationIconName, c"hushfold");
        let params = protocol::PlaybackStreamParams {
            sample_spec: SampleSpec {
                format: SampleFormat::Float32Le,
                channels: 2,
                sample_rate: SR,
            },
            channel_map: protocol::ChannelMap::stereo(),
            sink_name,
            buffer_attr: BufferAttr {
                target_length: QUEUED * FRAME,
                minimum_request_length: LEAST * FRAME,
                ..Default::default()
            },
            flags: StreamFlags {
                adjust_latency: true,
                ..Default::default()
            },
            props,
            ..Default::default()
        };
        // The server asks for bytes; frames are rendered a block at a time and
        // handed over as asked, what is left over kept for the next ask.
        let mut held: Vec<u8> = Vec::with_capacity(BLOCK * FRAME as usize);
        let mut at = 0;
        let fill = move |buf: &mut [u8]| {
            let mut n = 0;
            while n < buf.len() {
                if at == held.len() {
                    held.clear();
                    at = 0;
                    let mut feed = lock(&feed);
                    for _ in 0..BLOCK {
                        for s in feed.frame() {
                            held.extend_from_slice(&s.to_le_bytes());
                        }
                    }
                }
                let k = (buf.len() - n).min(held.len() - at);
                buf[n..n + k].copy_from_slice(&held[at..at + k]);
                (n, at) = (n + k, at + k);
            }
            buf.len()
        };
        let stream = pollster::block_on(self.client.create_playback_stream(params, fill.as_playback_source()))
            .map_err(|e| e.to_string())?;
        // The source never ends, so this waits until the stream is deleted or the
        // connection goes.
        let ended = Arc::new(AtomicBool::new(false));
        let (waiting, end) = (stream.clone(), ended.clone());
        let waiting = std::thread::Builder::new()
            .name("sound stream".into())
            .spawn(move || {
                let _ = pollster::block_on(waiting.play_all());
                end.store(true, Ordering::Relaxed);
            })
            .map_err(|e| e.to_string())?;
        Ok(Box::new(Open {
            stream,
            ended,
            waiting: Some(waiting),
            spoiled: self.spoiled.clone(),
        }))
    }
}

struct Open {
    stream: PlaybackStream,
    ended: Arc<AtomicBool>,
    waiting: Option<JoinHandle<()>>,
    spoiled: Arc<AtomicBool>,
}

impl Stream for Open {
    fn lost(&self) -> bool {
        self.ended.load(Ordering::Relaxed)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        // Its waiting thread holds it too, so it must be deleted, not just dropped;
        // and that thread must be done with it before the next stream is made. The
        // last of a stream's handles to go sends a delete of its own, and the server
        // gives a new stream the channel an old one had.
        match pollster::block_on(self.stream.clone().delete()) {
            Ok(()) => {
                if let Some(waiting) = self.waiting.take() {
                    let _ = waiting.join();
                }
            }
            Err(_) => self.spoiled.store(true, Ordering::Relaxed),
        }
    }
}
