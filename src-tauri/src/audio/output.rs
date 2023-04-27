use std::sync::{
    atomic::{AtomicBool, AtomicU8},
    Arc,
};

use super::resampler::Resampler;
use cpal::{traits::*, *};
use rb::*;
use symphonia::core::{
    audio::{Channels, RawSample, SignalSpec},
    conv::{ConvertibleSample, IntoSample},
};

pub trait AudioOutput {
    fn stream_config(&self) -> &StreamConfig;
    fn sample_format(&self) -> SampleFormat;
    fn stream(&self) -> &Stream;
    fn is_dead(&self) -> bool;
    fn stream_mut(&mut self) -> &mut Stream;
    fn set_volume(&mut self, volume: f64);
    fn volume(&self) -> f64;
    fn write(&mut self, decoded: symphonia::core::audio::AudioBufferRef<'_>);
    fn flush(&mut self);
}

pub struct AudioStreamPlayer<T: AudioOutputSample> {
    config: StreamConfig,
    sample_format: SampleFormat,
    stream: Stream,
    is_dead: Arc<AtomicBool>,
    prod: rb::Producer<T>,
    volume: Arc<AtomicU8>,
    resampler: Option<Resampler<T>>,
    resampler_duration: usize,
    resampler_spec: SignalSpec,
}

pub trait AudioOutputSample:
    SizedSample
    + ConvertibleSample
    + IntoSample<f32>
    + RawSample
    + std::marker::Send
    + Default
    + 'static
{
}

impl AudioOutputSample for i8 {}
impl AudioOutputSample for i16 {}
impl AudioOutputSample for i32 {}
// impl AudioOutputSample for i64 {}
impl AudioOutputSample for u8 {}
impl AudioOutputSample for u16 {}
impl AudioOutputSample for u32 {}
// impl AudioOutputSample for u64 {}
impl AudioOutputSample for f32 {}
impl AudioOutputSample for f64 {}

impl<T: AudioOutputSample> AudioOutput for AudioStreamPlayer<T> {
    fn stream_config(&self) -> &StreamConfig {
        &self.config
    }

    fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    fn stream(&self) -> &Stream {
        &self.stream
    }

    fn stream_mut(&mut self) -> &mut Stream {
        &mut self.stream
    }

    fn is_dead(&self) -> bool {
        self.is_dead.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn set_volume(&mut self, volume: f64) {
        let volume = (volume * 255.).clamp(0., 255.) as u8;
        self.volume
            .store(volume, std::sync::atomic::Ordering::SeqCst);
    }

    fn volume(&self) -> f64 {
        self.volume.load(std::sync::atomic::Ordering::SeqCst) as f64 / 255.
    }

    fn write(&mut self, decoded: symphonia::core::audio::AudioBufferRef<'_>) {
        if decoded.frames() == 0 {
            return;
        }

        let should_replace_resampler = self.resampler.is_none()
            || self.resampler_duration != decoded.capacity()
            || &self.resampler_spec != decoded.spec();

        if should_replace_resampler {
            self.resampler = Some(Resampler::<T>::new(
                *decoded.spec(),
                self.config.sample_rate.0 as _,
                decoded.capacity() as _,
            ));
            println!(
                "将会重采样 {}hz -> {}hz",
                decoded.spec().rate,
                self.config.sample_rate.0
            );
            self.resampler_duration = decoded.capacity();
            self.resampler_spec = *decoded.spec();
        }

        let rsp = self.resampler.as_mut().unwrap();

        if let Some(mut buf) = rsp.resample(decoded) {
            while let Some(written) = self.prod.write_blocking(buf) {
                buf = &buf[written..];
            }
        }
    }

    fn flush(&mut self) {}
}

fn init_audio_stream_inner<T: AudioOutputSample + Into<f64>>(
    output: Device,
    selected_config: StreamConfig,
) -> Box<dyn AudioOutput> {
    let ring_len =
        ((200 * selected_config.sample_rate.0 as usize) / 1000) * selected_config.channels as usize;
    let ring = rb::SpscRb::<T>::new(ring_len);
    let prod = ring.producer();
    let cons = ring.consumer();
    let is_dead = Arc::new(AtomicBool::new(false));
    let is_dead_c = is_dead.clone();
    let volume: Arc<_> = Arc::new(AtomicU8::new(u8::MAX >> 1));
    let volume_c = volume.clone();
    let stream = output
        .build_output_stream::<T, _, _>(
            &selected_config,
            move |data, _info| {
                let written = cons.read(data).unwrap_or(0);
                data[written..].fill(T::MID);
                let volume = volume_c.load(std::sync::atomic::Ordering::SeqCst) as f32 / 255.;
                data.iter_mut().for_each(|x| {
                    let s: f32 = (*x).into_sample();
                    *x = (s * volume).into_sample();
                });
            },
            move |err| {
                println!("[WARN][AT] {err}");
                is_dead_c.store(true, std::sync::atomic::Ordering::SeqCst);
            },
            None,
        )
        .unwrap();
    println!("音频输出流准备完毕！");
    Box::new(AudioStreamPlayer {
        config: selected_config,
        sample_format: <T as SizedSample>::FORMAT,
        stream,
        prod,
        is_dead,
        volume,
        resampler: None,
        resampler_duration: 0,
        resampler_spec: SignalSpec {
            rate: 0,
            channels: Channels::empty(),
        },
    })
}

pub fn init_audio_player(output_device_name: &str) -> Box<dyn AudioOutput> {
    let host = cpal::default_host();
    let output = if output_device_name.is_empty() {
        host.default_output_device().unwrap()
    } else {
        host.output_devices()
            .unwrap()
            .find(|d| d.name().unwrap_or_default() == output_device_name)
            .unwrap_or_else(|| host.default_output_device().unwrap())
    };
    println!(
        "已初始化输出音频设备为 {}",
        output.name().unwrap_or_default()
    );
    let configs = output
        .supported_output_configs()
        .unwrap()
        .collect::<Vec<_>>();
    let mut selected_config = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(0),
        buffer_size: cpal::BufferSize::Default,
    };
    let mut selected_sample_format = SampleFormat::F32;
    for config in configs {
        println!(
            "已找到配置 {}hz-{}hz {} 通道 {}",
            config.min_sample_rate().0,
            config.max_sample_rate().0,
            config.channels(),
            config.sample_format()
        );
        if config.channels() > selected_config.channels
            || config.min_sample_rate().0 > selected_config.sample_rate.0
        {
            selected_config.channels = config.channels();
            selected_config.sample_rate.0 = config.min_sample_rate().0;
            selected_sample_format = config.sample_format();
        }
    }
    println!(
        "尝试通过配置 {}hz {} 通道 创建输出流",
        selected_config.sample_rate.0, selected_config.channels,
    );
    match selected_sample_format {
        SampleFormat::I8 => init_audio_stream_inner::<i8>(output, selected_config),
        SampleFormat::I16 => init_audio_stream_inner::<i16>(output, selected_config),
        SampleFormat::I32 => init_audio_stream_inner::<i32>(output, selected_config),
        // SampleFormat::I64 => init_audio_stream_inner::<i64>(output, selected_config),
        SampleFormat::U8 => init_audio_stream_inner::<u8>(output, selected_config),
        SampleFormat::U16 => init_audio_stream_inner::<u16>(output, selected_config),
        SampleFormat::U32 => init_audio_stream_inner::<u32>(output, selected_config),
        // SampleFormat::U64 => init_audio_stream_inner::<u64>(output, selected_config),
        SampleFormat::F32 => init_audio_stream_inner::<f32>(output, selected_config),
        SampleFormat::F64 => init_audio_stream_inner::<f64>(output, selected_config),
        _ => unreachable!(),
    }
}
