use cpal::traits::*;
use std::time::Duration;
use std::{io::*, sync::atomic::AtomicU16};
use std::{sync::atomic::AtomicUsize, thread::spawn};
use std::{
    sync::{
        atomic::AtomicBool,
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
};
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::{
    codecs::Decoder,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    probe::ProbeResult,
    units::TimeBase,
};
use tauri::Manager;

use output::*;

mod output;
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

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Default, Clone)]
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
    #[serde(rename_all = "camelCase")]
    SyncStatus,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "data")]
pub enum AudioThreadEvent {
    #[serde(rename_all = "camelCase")]
    PlayPosition { position: f64 },
    #[serde(rename_all = "camelCase")]
    LoadPosition { position: f64 },
    #[serde(rename_all = "camelCase")]
    LoadAudio { ncm_id: String, duration: f64 },
    #[serde(rename_all = "camelCase")]
    LoadingAudio { ncm_id: String },
    #[serde(rename_all = "camelCase")]
    SyncStatus {
        ncm_id: String,
        is_playing: bool,
        duration: f64,
        position: f64,
        load_position: f64,
    },
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
            AudioThreadMessage::SyncStatus { .. } => "",
        }
    }
}

static MSG_SENDER: Mutex<Option<Sender<AudioThreadMessage>>> = Mutex::new(None);

pub fn stop_audio_thread() {
    (*MSG_SENDER.lock().unwrap()) = None;
}

fn send_msg_to_audio_thread_inner(msg: AudioThreadMessage) -> std::result::Result<(), String> {
    let sx = MSG_SENDER.lock().map_err(|x| x.to_string())?;
    let sx = sx
        .as_ref()
        .ok_or_else(|| "线程消息通道未建立".to_string())?;
    sx.send(msg).map_err(|x| x.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn send_msg_to_audio_thread(msg: AudioThreadMessage) -> std::result::Result<(), String> {
    send_msg_to_audio_thread_inner(msg)
}

pub fn audio_thread_main(app: tauri::AppHandle, rx: Receiver<AudioThreadMessage>) {
    println!("音频线程已开始运行！");
    let codecs = symphonia::default::get_codecs();
    let probe = symphonia::default::get_probe();
    let mut player = output::init_audio_player();
    let audio_cache_dir = app
        .path_resolver()
        .app_cache_dir()
        .unwrap()
        .join("audio-cache");
    let mut is_playing = false;
    let mut session = attohttpc::Session::new();
    session.header("origin", "orpheus://orpheus");
    session.header("user-agent", "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.7.200791");
    let audio_current_tmp_file = audio_cache_dir.join("audio_tmp");
    let _ = std::fs::create_dir_all(audio_cache_dir);

    let mut playlist = Vec::<SongData>::with_capacity(4096);
    let mut current_play_index = 0usize;
    let mut current_song = SongData::default();
    let stop_download_atom = Arc::new(AtomicBool::new(false));
    let full_downloaded_atom = Arc::new(AtomicBool::new(false));
    let load_position = Arc::new(AtomicU16::new(0));
    let mut download_thread_handle: Option<JoinHandle<()>> = None;
    let mut format_result: Option<ProbeResult> = None;
    let mut decoder: Option<Box<dyn Decoder>> = None;
    let mut timebase = TimeBase::default();
    let mut play_position = 0.;
    let mut play_duration = 0.;

    loop {
        if is_playing {
            for msg in rx.try_iter() {
                match &msg {
                    AudioThreadMessage::SetCookie { cookie, .. } => {
                        session.header("cookie", cookie);
                        println!("已设置 Cookie 头为 {cookie}");
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::ResumeAudio { .. } => {
                        is_playing = true;
                        println!("开始继续播放歌曲！");
                        player.stream().play().unwrap();
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::PauseAudio { .. } => {
                        is_playing = false;
                        player.stream().pause().unwrap();
                        println!("播放已暂停！");
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::NextSong { .. } => {
                        format_result = None;
                        decoder = None;
                        is_playing = true;
                        player.stream().play().unwrap();
                        println!("播放下一首歌曲！");
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::JumpToSong { song_index, .. } => {
                        format_result = None;
                        decoder = None;
                        is_playing = true;
                        if *song_index == 0 {
                            current_play_index = playlist.len();
                        } else {
                            current_play_index = *song_index - 1;
                        }
                        player.stream().play().unwrap();
                        println!("播放第 {} 首歌曲！", *song_index + 1);
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::SetPlaylist { songs, .. } => {
                        playlist = songs.to_owned();
                        println!("已设置播放列表，歌曲数量为 {}", songs.len());
                        current_play_index = playlist.len();
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::SyncStatus => {
                        let _ = app.emit_all(
                            "on-audio-thread-event",
                            AudioThreadEvent::SyncStatus {
                                ncm_id: current_song.ncm_id.to_owned(),
                                is_playing,
                                duration: play_duration,
                                position: play_position,
                                load_position: load_position
                                    .load(std::sync::atomic::Ordering::SeqCst)
                                    as f64
                                    / u16::MAX as f64,
                            },
                        );
                    }
                    other => dbg!(other).ret(&app, None::<()>).unwrap(),
                }
            }
            // 处理音乐播放
            let mut is_song_finished = false;
            if let Some(format_result) = format_result.as_mut() {
                if let Some(decoder) = decoder.as_mut() {
                    match format_result.format.next_packet() {
                        Ok(packet) => match decoder.decode(&packet) {
                            Ok(buf) => {
                                let time = timebase.calc_time(packet.ts);
                                play_position = time.seconds as f64 + time.frac;
                                let _ = app.emit_all(
                                    "on-audio-thread-event",
                                    AudioThreadEvent::PlayPosition {
                                        position: play_position,
                                    },
                                );
                                player.write(buf);
                            }
                            Err(err) => {
                                println!("[WARN][AT] 解码器解码出错 {err}");
                            }
                        },
                        Err(DecodeError::IoError(err)) => match err.kind() {
                            ErrorKind::UnexpectedEof => {
                                if full_downloaded_atom.load(core::sync::atomic::Ordering::SeqCst) {
                                    is_song_finished = true;
                                }
                            }
                            _ => {
                                is_song_finished = true;
                            }
                        },
                        Err(_) => {
                            is_song_finished = true;
                        }
                    }
                } else {
                    let track = format_result.format.default_track().unwrap();
                    timebase = track.codec_params.time_base.unwrap_or_default();
                    decoder = codecs.make(&track.codec_params, &Default::default()).ok();
                    let duration =
                        timebase.calc_time(track.codec_params.n_frames.unwrap_or_default());
                    play_duration = duration.seconds as f64 + duration.frac;
                    let _ = app.emit_all(
                        "on-audio-thread-event",
                        AudioThreadEvent::LoadAudio {
                            ncm_id: current_song.ncm_id.to_owned(),
                            duration: play_duration,
                        },
                    );
                }
            } else {
                // 选择下一首歌
                if playlist.is_empty() {
                    is_playing = false;
                } else {
                    // 如果存在则中断正在流式播放的歌曲下载线程
                    stop_download_atom.store(true, core::sync::atomic::Ordering::SeqCst);
                    if let Some(handle) = download_thread_handle.take() {
                        let _ = handle.join();
                    }
                    stop_download_atom.store(false, core::sync::atomic::Ordering::SeqCst);
                    // 选歌
                    current_play_index += 1;
                    if current_play_index >= playlist.len() {
                        current_play_index = 0;
                    }
                    current_song = playlist[current_play_index].to_owned();
                    println!(
                        "即将尝试播放下一首歌：{} ({})",
                        current_song.ncm_id, current_song.local_file
                    );
                    let _ = app.emit_all(
                        "on-audio-thread-event",
                        AudioThreadEvent::LoadingAudio {
                            ncm_id: current_song.ncm_id.to_owned(),
                        },
                    );
                    // 是否有本地文件
                    if let Ok(file) = std::fs::OpenOptions::new()
                        .read(true)
                        .open(&current_song.local_file)
                    {
                        full_downloaded_atom.store(true, core::sync::atomic::Ordering::SeqCst);
                        let source_stream = MediaSourceStream::new(
                            Box::new(file),
                            MediaSourceStreamOptions::default(),
                        );
                        format_result = probe
                            .format(
                                &Default::default(),
                                source_stream,
                                &Default::default(),
                                &Default::default(),
                            )
                            .ok();
                    } else {
                        // 准备联网获取
                        let post_data = format!(
                            "{{\"ids\":\"[{}]\",\"level\":\"hires\",\"encodeType\":\"flac\"}}",
                            current_song.ncm_id
                        );
                        let res = session
                            .post("https://interface.music.163.com/eapi/song/enhance/player/url/v1")
                            .header("content-type", "application/x-www-form-urlencoded")
                            .bytes(
                                concat_string::concat_string!(
                                    "params=",
                                    crate::eapi::eapi_encrypt_for_request(
                                        "/api/song/enhance/player/url/v1",
                                        &post_data
                                    )
                                )
                                .as_bytes(),
                            )
                            .send()
                            .unwrap()
                            .json::<NCMResponse<Vec<NCMSongResponse>>>()
                            .unwrap();
                        let song_url = res
                            .data
                            .as_ref()
                            .map(|x| {
                                x.first()
                                    .map(|y| y.url.to_owned().unwrap_or_default())
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default();
                        let song_size = res
                            .data
                            .as_ref()
                            .map(|x| x.first().map(|y| y.size).unwrap_or_default())
                            .unwrap_or_default();
                        if song_url.is_empty() {
                            // 歌曲链接不存在
                            is_song_finished = true;
                            println!("未找到音乐下载链接，跳过");
                        } else {
                            println!("正在流式播放 {song_url}");
                            let mut output_file = std::fs::OpenOptions::new()
                                .create(true)
                                .truncate(true)
                                .write(true)
                                .open(&audio_current_tmp_file)
                                .unwrap();
                            let mut output_file_reader = std::fs::OpenOptions::new()
                                .read(true)
                                .open(&audio_current_tmp_file)
                                .unwrap();
                            let mut song_res = session.get(song_url).send().unwrap();
                            let stop_downloaded_atom = stop_download_atom.clone();
                            let full_downloaded_atom = full_downloaded_atom.clone();
                            let _full_downloaded_atom = full_downloaded_atom.clone();
                            let download_size_atom = Arc::new(AtomicUsize::new(0));
                            let _download_size_atom = download_size_atom.clone();
                            let _app = app.clone();
                            download_thread_handle = Some(spawn(move || {
                                let mut buf = [0u8; 1024];
                                while let Ok(size) = song_res.read(&mut buf) {
                                    let should_stopped = stop_downloaded_atom
                                        .load(core::sync::atomic::Ordering::SeqCst);
                                    if size == 0 || should_stopped {
                                        if should_stopped {
                                            println!("音频下载中断");
                                        } else {
                                            println!("音频下载完成");
                                        }
                                        break;
                                    } else {
                                        output_file.write_all(&buf[..size]).unwrap();
                                        if _download_size_atom
                                            .fetch_add(size, core::sync::atomic::Ordering::SeqCst)
                                            == 0
                                        {
                                            output_file.sync_all().unwrap();
                                        }
                                    }
                                }
                                _full_downloaded_atom
                                    .store(true, core::sync::atomic::Ordering::SeqCst);
                            }));
                            // 将头部下载下来，以确认格式
                            while download_size_atom.load(std::sync::atomic::Ordering::SeqCst)
                                < song_size.min(1024 * 16)
                            {}
                            loop {
                                output_file_reader.rewind().unwrap();
                                let source_stream = MediaSourceStream::new(
                                    Box::new(output_file_reader.try_clone().unwrap()),
                                    MediaSourceStreamOptions::default(),
                                );
                                match probe.format(
                                    &Default::default(),
                                    source_stream,
                                    &Default::default(),
                                    &Default::default(),
                                ) {
                                    Ok(result) => {
                                        format_result = Some(result);
                                        break;
                                    }
                                    Err(err) => match err {
                                        DecodeError::Unsupported(_)
                                        | DecodeError::DecodeError(_)
                                        | DecodeError::IoError(_) => {
                                            if full_downloaded_atom
                                                .load(std::sync::atomic::Ordering::SeqCst)
                                            {
                                                is_song_finished = true;
                                                break;
                                            }
                                        }
                                        _ => {
                                            is_song_finished = true;
                                            break;
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }
            if is_song_finished {
                format_result = None;
                decoder = None;
            }
        } else {
            match rx.recv() {
                Ok(msg) => match &msg {
                    AudioThreadMessage::SetCookie { cookie, .. } => {
                        session.header("cookie", cookie);
                        println!("已设置 Cookie 头为 {cookie}");
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::ResumeAudio { .. } => {
                        is_playing = true;
                        println!("开始继续播放歌曲！");
                        player.stream().play().unwrap();
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::PauseAudio { .. } => {
                        is_playing = false;
                        player.stream().pause().unwrap();
                        println!("播放已暂停！");
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::NextSong { .. } => {
                        format_result = None;
                        decoder = None;
                        is_playing = true;
                        player.stream().play().unwrap();
                        println!("播放下一首歌曲！");
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::JumpToSong { song_index, .. } => {
                        format_result = None;
                        decoder = None;
                        is_playing = true;
                        if *song_index == 0 {
                            current_play_index = playlist.len();
                        } else {
                            current_play_index = *song_index - 1;
                        }
                        player.stream().play().unwrap();
                        println!("播放第 {} 首歌曲！", *song_index + 1);
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::SetPlaylist { songs, .. } => {
                        playlist = songs.to_owned();
                        println!("已设置播放列表，歌曲数量为 {}", songs.len());
                        current_play_index = playlist.len();
                        msg.ret(&app, None::<()>).unwrap();
                    }
                    AudioThreadMessage::SyncStatus => {
                        let _ = app.emit_all(
                            "on-audio-thread-event",
                            AudioThreadEvent::SyncStatus {
                                ncm_id: current_song.ncm_id.to_owned(),
                                is_playing,
                                duration: play_duration,
                                position: play_position,
                                load_position: load_position
                                    .load(std::sync::atomic::Ordering::SeqCst)
                                    as f64
                                    / u16::MAX as f64,
                            },
                        );
                    }
                    other => dbg!(other).ret(&app, None::<()>).unwrap(),
                },
                _ => break,
            }
        }
    }
    (*MSG_SENDER.lock().unwrap()) = None;
    println!("音频线程已结束运行！");
}

#[tauri::command]
pub async fn init_audio_thread(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let mut sender = MSG_SENDER.lock().map_err(|x| x.to_string())?;
    if sender.is_none() {
        let (sx, rx) = channel::<AudioThreadMessage>();
        (*sender) = Some(sx);
        spawn(move || {
            audio_thread_main(app, rx);
        });
    } else {
        drop(sender);
        send_msg_to_audio_thread_inner(AudioThreadMessage::SyncStatus)?;
    }
    Ok(())
}

#[test]
fn test_audio() {
    let data = format!(
        "{{\"ids\":\"[{}]\",\"level\":\"hires\",\"encodeType\":\"flac\"}}",
        41647509usize
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
        .header("content-type", "application/x-www-form-urlencoded")
        .bytes(
            concat_string::concat_string!(
                "params=",
                crate::eapi::eapi_encrypt_for_request("/api/song/enhance/player/url/v1", &data)
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
    let full_downloaded = Arc::new(AtomicBool::new(false));
    let _full_downloaded = full_downloaded.clone();
    spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(size) = song_res.read(&mut buf) {
            if size == 0 {
                break;
            } else {
                output_file.write_all(&buf[..size]).unwrap();
            }
        }
        _full_downloaded.store(true, core::sync::atomic::Ordering::SeqCst);
        println!("音频下载完成");
    });
    let mut player = output::init_audio_player();
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
    let mut decoder = codecs
        .make(&track.codec_params, &Default::default())
        .unwrap();
    println!("开始播放！");
    player.stream().play().unwrap();
    loop {
        use std::io::ErrorKind;
        use symphonia::core::errors::Error as DecodeError;
        match format_result.format.next_packet() {
            Ok(packet) => match decoder.decode(&packet) {
                Ok(buf) => {
                    player.write(buf);
                }
                Err(err) => {
                    println!("[WARN][AT] 解码器解码出错 {err}");
                }
            },
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
    println!("播放完毕！");
}
