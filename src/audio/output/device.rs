//! The device's way, through cpal: WASAPI, CoreAudio, or ALSA where there is no
//! sound server. A device is opened as it is set (its rate, channels and sample
//! format), never set to suit the game, so nothing else playing on it is changed;
//! the engine's frames are resampled and spread to fit.

use super::{Device, Feed, Resample, Shared, Stream, lock};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, ErrorKind, FromSample, SampleFormat, SizedSample, StreamConfig, SupportedBufferSize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Frames asked for at a time, where the device lets them be chosen (23 ms).
const FRAMES: u32 = 1024;

pub struct Devices {
    host: cpal::Host,
}

impl Devices {
    pub fn new() -> Self {
        Devices {
            host: cpal::default_host(),
        }
    }

    /// The outputs worth offering. ALSA lists every plugin and every way into each
    /// card; of those, each card's `default:` device is offered.
    pub fn list(&self) -> (Vec<Device>, Option<String>) {
        let devices = (self.host.output_devices().into_iter().flatten())
            .filter_map(|d| {
                let id = d.id().ok()?.to_string();
                if id.strip_prefix("alsa:").is_some_and(|pcm| !pcm.starts_with("default:")) {
                    return None;
                }
                let name = d.description().ok()?.name().to_string();
                Some(Device { id, name })
            })
            .collect();
        let system = self.host.default_output_device().and_then(|d| d.description().ok());
        (devices, system.map(|d| d.name().to_string()))
    }

    /// Found among those listed, by the id they were listed with (cpal's ALSA
    /// `device_by_id` names a card's `default:` device otherwise than it lists it).
    fn find(&self, id: &str) -> Option<cpal::Device> {
        let mut all = self.host.output_devices().into_iter().flatten();
        all.find(|d| d.id().is_ok_and(|d| d.to_string() == id))
    }

    pub fn has(&self, id: &str) -> bool {
        self.find(id).is_some()
    }

    pub fn system_id(&self) -> Option<String> {
        Some(self.host.default_output_device()?.id().ok()?.to_string())
    }

    /// A stream on `id`, or (None) on the system's output.
    pub fn open(&self, id: Option<&str>, feed: &Shared<Feed>) -> Result<Box<dyn Stream>, String> {
        if let Some(id) = id {
            let device = self.find(id).ok_or("not there")?;
            return start(&device, feed, None);
        }
        let system = self.system_id();
        // ALSA's own default may be a card the sound server holds, or one it isn't
        // using, taken from under it; PipeWire's ALSA device, where there is one,
        // goes through it.
        #[cfg(target_os = "linux")]
        if let Some(device) = self.find("alsa:pipewire")
            && let Ok(s) = start(&device, feed, system.clone())
        {
            return Ok(s);
        }
        let device = self.host.default_output_device().ok_or("no output device")?;
        start(&device, feed, system)
    }
}

struct Open {
    _stream: cpal::Stream,
    lost: Arc<AtomicBool>,
    system: Option<String>,
}

impl Stream for Open {
    fn lost(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }

    fn system(&self) -> Option<&str> {
        self.system.as_deref()
    }
}

/// Play on `device` as it is set; `system` is the system output's id, if that is
/// what was asked for.
fn start(device: &cpal::Device, feed: &Shared<Feed>, system: Option<String>) -> Result<Box<dyn Stream>, String> {
    let set = device.default_output_config().map_err(|e| e.to_string())?;
    let buffer_size = match set.buffer_size() {
        SupportedBufferSize::Range { min, max } => BufferSize::Fixed(FRAMES.clamp(*min, *max)),
        SupportedBufferSize::Unknown => BufferSize::Default,
    };
    let config = StreamConfig {
        channels: set.channels(),
        sample_rate: set.sample_rate(),
        buffer_size,
    };
    let lost = Arc::new(AtomicBool::new(false));
    let (f, l) = (feed.clone(), lost.clone());
    let stream = match set.sample_format() {
        SampleFormat::F32 => build::<f32>(device, config, f, l),
        SampleFormat::F64 => build::<f64>(device, config, f, l),
        SampleFormat::I16 => build::<i16>(device, config, f, l),
        SampleFormat::I24 => build::<cpal::I24>(device, config, f, l),
        SampleFormat::I32 => build::<i32>(device, config, f, l),
        SampleFormat::U8 => build::<u8>(device, config, f, l),
        SampleFormat::U16 => build::<u16>(device, config, f, l),
        other => return Err(format!("{other} samples")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let name = device.description().map_or(String::new(), |d| d.name().to_string());
    let (rate, channels, format) = (config.sample_rate, config.channels, set.sample_format());
    eprintln!("hushfold: sound on {name} ({rate} Hz, {channels} channels, {format})");
    Ok(Box::new(Open {
        _stream: stream,
        lost,
        system,
    }))
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: StreamConfig,
    feed: Shared<Feed>,
    lost: Arc<AtomicBool>,
) -> Result<cpal::Stream, cpal::Error> {
    let mut resample = Resample::new(config.sample_rate);
    let channels = usize::from(config.channels);
    let fill = move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
        let mut feed = lock(&feed);
        for frame in out.chunks_mut(channels) {
            let [l, r] = resample.frame(&mut feed);
            match frame {
                [m] => *m = T::from_sample(0.5 * (l + r)),
                [a, b, rest @ ..] => {
                    (*a, *b) = (T::from_sample(l), T::from_sample(r));
                    rest.fill(T::EQUILIBRIUM);
                }
                [] => {}
            }
        }
    };
    let failed = move |e: cpal::Error| {
        let ends = match e.kind() {
            ErrorKind::Xrun | ErrorKind::DeviceChanged | ErrorKind::RealtimeDenied => false,
            ErrorKind::DeviceNotAvailable | ErrorKind::StreamInvalidated => true,
            // ALSA's stream plays on after any other; WASAPI's stops at it.
            _ => cfg!(windows),
        };
        if ends {
            lost.store(true, Ordering::Relaxed);
        }
    };
    device.build_output_stream(config, fill, failed, None)
}
