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
    fn stream_mut(&mut self) -> &mut Stream;
    fn write(&mut self, decoded: symphonia::core::audio::AudioBufferRef<'_>);
    fn flush(&mut self);
}

pub struct AudioStreamPlayer<T: AudioOutputSample> {
    config: StreamConfig,
    sample_format: SampleFormat,
    stream: Stream,
    prod: rb::Producer<T>,
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

fn init_audio_stream_inner<T: AudioOutputSample>(
    output: Device,
    selected_config: StreamConfig,
) -> Box<dyn AudioOutput> {
    let ring_len =
        ((200 * selected_config.sample_rate.0 as usize) / 1000) * selected_config.channels as usize;
    let ring = rb::SpscRb::<T>::new(ring_len);
    let prod = ring.producer();
    let cons = ring.consumer();
    let stream = output
        .build_output_stream::<T, _, _>(
            &selected_config,
            move |data, _info| {
                let written = cons.read(data).unwrap_or(0);
                data[written..].fill(T::MID);
            },
            |err| {
                println!("[WARN][AT] {err}");
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
        resampler: None,
        resampler_duration: 0,
        resampler_spec: SignalSpec {
            rate: 0,
            channels: Channels::empty(),
        },
    })
}

pub fn init_audio_player() -> Box<dyn AudioOutput> {
    let output = cpal::default_host()
        .output_devices()
        .unwrap()
        .next()
        .unwrap();
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
