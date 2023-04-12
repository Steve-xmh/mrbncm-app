use alloc::sync::Arc;
use core::sync::atomic::AtomicBool;
use cpal::{traits::*, SampleFormat, SampleRate, StreamConfig};
use std::thread::spawn;
use std::{
    io::{Read, Write},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
};
use std::{sync::mpsc::RecvTimeoutError, time::Duration};
use symphonia::core::{
    audio::SignalSpec,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    probe::Hint,
};
use tauri::Manager;

use crate::eapi::{eapi_decrypt, eapi_encrypt_for_request};

mod resampler;

#[derive(serde::Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct NCMResponse<T> {
    data: Option<T>,
    code: i32,
}

#[derive(serde::Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NCMSongResponse {
    pub id: usize,
    pub url: Option<String>,
    pub br: usize,
    pub size: usize,
    pub md5: Option<String>,
    #[serde(rename = "type")]
    pub audio_type: Option<String>,
    pub encode_type: Option<String>,
    pub time: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SongData {
    pub ncm_id: String,
    pub local_file: String,
    pub duration: usize,
    pub orig_order: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum AudioThreadMessage {
    #[serde(rename_all = "camelCase")]
    ResumeAudio { callback_id: String },
    #[serde(rename_all = "camelCase")]
    PauseAudio { callback_id: String },
    #[serde(rename_all = "camelCase")]
    SeekAudio {
        callback_id: String,
        position: Duration,
    },
    #[serde(rename_all = "camelCase")]
    JumpToSong {
        callback_id: String,
        song_index: usize,
    },
    #[serde(rename_all = "camelCase")]
    PrevSong { callback_id: String },
    #[serde(rename_all = "camelCase")]
    NextSong { callback_id: String },
    #[serde(rename_all = "camelCase")]
    SetPlaylist {
        callback_id: String,
        songs: Vec<SongData>,
    },
    #[serde(rename_all = "camelCase")]
    SetCookie { callback_id: String, cookie: String },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioThreadEventMessage<T> {
    callback_id: String,
    data: T,
}

impl AudioThreadMessage {
    pub fn ret(
        &self,
        app: &tauri::AppHandle,
        data: impl serde::Serialize + Clone,
    ) -> tauri::Result<()> {
        app.emit_all(
            "on_audio_thread_message",
            AudioThreadEventMessage {
                callback_id: self.callback_id().to_owned(),
                data,
            },
        )
    }

    pub fn callback_id(&self) -> &str {
        match &self {
            AudioThreadMessage::ResumeAudio { callback_id } => callback_id.as_str(),
            AudioThreadMessage::PauseAudio { callback_id } => callback_id.as_str(),
            AudioThreadMessage::SeekAudio { callback_id, .. } => callback_id.as_str(),
            AudioThreadMessage::JumpToSong { callback_id, .. } => callback_id.as_str(),
            AudioThreadMessage::PrevSong { callback_id } => callback_id.as_str(),
            AudioThreadMessage::NextSong { callback_id } => callback_id.as_str(),
            AudioThreadMessage::SetPlaylist { callback_id, .. } => callback_id.as_str(),
            AudioThreadMessage::SetCookie { callback_id, .. } => callback_id.as_str(),
        }
    }
}

static MSG_SENDER: Mutex<Option<Sender<AudioThreadMessage>>> = Mutex::new(None);

pub fn stop_audio_thread() {
    (*MSG_SENDER.lock().unwrap()) = None;
}

#[tauri::command]
pub async fn send_msg_to_audio_thread(msg: AudioThreadMessage) -> Result<(), String> {
    let sx = MSG_SENDER.lock().map_err(|x| x.to_string())?;
    let sx = sx
        .as_ref()
        .ok_or_else(|| "线程消息通道未建立".to_string())?;
    sx.send(msg).map_err(|x| x.to_string())?;
    Ok(())
}

fn init_audio_stream() -> (StreamConfig, SampleFormat, cpal::Stream) {
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
    let stream = output
        .build_output_stream_raw(
            &selected_config,
            selected_sample_format,
            |data, info| {},
            |err| {
                println!("[WARN][AT] {err}");
            },
            None,
        )
        .unwrap();
    println!("音频输出流准备完毕！");
    return (selected_config, selected_sample_format, stream);
}

pub fn audio_thread_main(app: tauri::AppHandle, rx: Receiver<AudioThreadMessage>) {
    println!("音频线程已开始运行！");
    let (config, stream_format, stream) = init_audio_stream();
    let audio_cache_dir = app
        .path_resolver()
        .app_cache_dir()
        .unwrap()
        .join("audio-cache");
    let mut is_playing = false;
    let audio_current_tmp_file = audio_cache_dir.join("audio_tmp");
    let _ = std::fs::create_dir_all(audio_cache_dir);
    let mut current_cookie = String::new();
    let mut playlist = Vec::<SongData>::with_capacity(4096);
    let mut current_play_index = 0usize;
    let mut current_song = SongData::default();
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(msg) => match &msg {
                AudioThreadMessage::SetCookie { cookie, .. } => {
                    current_cookie = cookie.to_owned();
                    println!("已设置 Cookie 头为 {}", cookie);
                    msg.ret(&app, None::<()>).unwrap();
                }
                AudioThreadMessage::ResumeAudio { .. } => {
                    is_playing = true;
                    msg.ret(&app, None::<()>).unwrap();
                }
                AudioThreadMessage::PauseAudio { .. } => {
                    is_playing = false;
                    msg.ret(&app, None::<()>).unwrap();
                }
                AudioThreadMessage::SetPlaylist { songs, .. } => {
                    playlist = songs.to_owned();
                    msg.ret(&app, None::<()>).unwrap();
                }
                other => dbg!(other).ret(&app, None::<()>).unwrap(),
            },
            Err(RecvTimeoutError::Timeout) => {}
            _ => break,
        }
        if is_playing {}
    }
    (*MSG_SENDER.lock().unwrap()) = None;
    println!("音频线程已结束运行！");
}

#[tauri::command]
pub async fn init_audio_thread(app: tauri::AppHandle) -> Result<(), String> {
    let mut sender = MSG_SENDER.lock().map_err(|x| x.to_string())?;
    if sender.is_none() {
        let (sx, rx) = channel::<AudioThreadMessage>();
        (*sender) = Some(sx);
        spawn(move || {
            audio_thread_main(app, rx);
        });
    }
    Ok(())
}

#[test]
fn test_audio() {
    let data = format!(
        "{{\"ids\":\"[{}]\",\"level\":\"hires\",\"encodeType\":\"flac\"}}",
        1994955842usize
    );
    println!("{data}");
    let mut session = attohttpc::Session::new();
    session.header("content-type", "application/x-www-form-urlencoded");
    session.header("origin", "orpheus://orpheus");
    session.header("user-agent", "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.7.200791");
    session.header(
        "cookie",
        dbg!(std::option_env!("NCM_COOKIE").unwrap_or_default()),
    );
    let res = session
        .post("https://interface.music.163.com/eapi/song/enhance/player/url/v1")
        .bytes(
            concat_string::concat_string!(
                "params=",
                eapi_encrypt_for_request("/api/song/enhance/player/url/v1", &data)
            )
            .as_bytes(),
        )
        .send()
        .unwrap()
        // .text()
        .json::<NCMResponse<Vec<NCMSongResponse>>>()
        .unwrap();
    // println!("{res}");
    let song_url = dbg!(res)
        .data
        .map(|x| {
            x.first()
                .map(|y| y.url.to_owned().unwrap_or_default())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    println!("song url: {song_url}");
    let mut song_res = session.get(song_url).send().unwrap();
    let mut output_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("music.flac")
        .unwrap();
    let output_file_reader = std::fs::OpenOptions::new()
        .read(true)
        .open("music.flac")
        .unwrap();
    let mut full_downloaded = Arc::new(AtomicBool::new(false));
    spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(size) = song_res.read(&mut buf) {
            if size == 0 {
                break;
            } else {
                output_file.write_all(&buf[..size]).unwrap();
            }
        }
        full_downloaded.store(true, core::sync::atomic::Ordering::SeqCst);
    });
    let (config, stream_format, stream) = init_audio_stream();
    let source_stream = MediaSourceStream::new(
        Box::new(output_file_reader),
        MediaSourceStreamOptions::default(),
    );
    let codecs = symphonia::default::get_codecs();
    let probe = symphonia::default::get_probe();
    let mut format_result = probe
        .format(
            &Default::default(),
            source_stream,
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
    let track = format_result.format.default_track().unwrap();
    let decoder = codecs.make(&track.codec_params, &Default::default()).unwrap();
    loop {
        use std::io::ErrorKind;
        use symphonia::core::errors::Error as DecodeError;
        match format_result.format.next_packet() {
            Ok(packet) => {}
            Err(DecodeError::IoError(err)) => match err.kind() {
                ErrorKind::UnexpectedEof => {
                    if full_downloaded.load(core::sync::atomic::Ordering::SeqCst) {
                        break;
                    }
                }
                _ => break,
            },
            Err(_) => break,
        }
    }
}
