use std::{
    io::{ErrorKind, Read, Seek, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU16, AtomicUsize},
        Arc,
    },
    thread::{spawn, JoinHandle},
};

use attohttpc::Session;
use cpal::traits::StreamTrait;
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::{
    codecs::{CodecRegistry, Decoder},
    io::{MediaSourceStream, MediaSourceStreamOptions},
    probe::{Probe, ProbeResult},
    units::TimeBase,
};
use tauri::Manager;

use crate::audio::{AudioThreadEvent, NCMResponse, NCMSongResponse};

use super::{output::AudioOutput, AudioThreadMessage, SongData};

pub struct AudioPlayer {
    app: tauri::AppHandle,
    codecs: &'static CodecRegistry,
    probe: &'static Probe,
    player: Box<dyn AudioOutput>,
    is_playing: bool,
    session: Session,
    audio_current_tmp_file: PathBuf,

    playlist: Vec<SongData>,
    current_play_index: usize,
    current_song: SongData,
    stop_download_atom: Arc<AtomicBool>,
    full_downloaded_atom: Arc<AtomicBool>,
    load_position: Arc<AtomicU16>,
    download_thread_handle: Option<JoinHandle<()>>,
    format_result: Option<ProbeResult>,
    decoder: Option<Box<dyn Decoder>>,
    timebase: TimeBase,
    play_position: f64,
    play_duration: f64,
}

impl AudioPlayer {
    pub fn new(app: tauri::AppHandle) -> Self {
        let codecs = symphonia::default::get_codecs();
        let probe = symphonia::default::get_probe();
        let player = super::output::init_audio_player();
        let audio_cache_dir = app
            .path_resolver()
            .app_cache_dir()
            .unwrap()
            .join("audio-cache");
        let mut session = attohttpc::Session::new();
        session.header("origin", "orpheus://orpheus");
        session.header("user-agent", "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.7.200791");
        let audio_current_tmp_file = audio_cache_dir.join("audio_tmp");
        let _ = std::fs::create_dir_all(audio_cache_dir);

        let playlist = Vec::<SongData>::with_capacity(4096);
        let current_song = SongData::default();
        let stop_download_atom = Arc::new(AtomicBool::new(false));
        let full_downloaded_atom = Arc::new(AtomicBool::new(false));
        let load_position = Arc::new(AtomicU16::new(0));
        let download_thread_handle: Option<JoinHandle<()>> = None;
        let format_result: Option<ProbeResult> = None;
        let decoder: Option<Box<dyn Decoder>> = None;
        let timebase = TimeBase::default();

        Self {
            app,
            codecs,
            probe,
            player,
            session,
            audio_current_tmp_file,
            playlist,
            current_song,
            stop_download_atom,
            full_downloaded_atom,
            load_position,
            download_thread_handle,
            format_result,
            decoder,
            timebase,
            is_playing: false,
            current_play_index: 0,
            play_position: 0.,
            play_duration: 0.,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn process_message(&mut self, msg: AudioThreadMessage) {
        match &msg {
            AudioThreadMessage::SetCookie { cookie, .. } => {
                self.session.header("cookie", cookie);
                println!("已设置 Cookie 头为 {cookie}");
                msg.ret(&self.app, None::<()>).unwrap();
            }
            AudioThreadMessage::ResumeAudio { .. } => {
                self.is_playing = true;
                println!("开始继续播放歌曲！");
                self.player.stream().play().unwrap();
                let _ = self.app.emit_all(
                    "on-audio-thread-event",
                    AudioThreadEvent::PlayStatus {
                        is_playing: self.is_playing,
                    },
                );
                msg.ret(&self.app, None::<()>).unwrap();
            }
            AudioThreadMessage::PauseAudio { .. } => {
                self.is_playing = false;
                self.player.stream().pause().unwrap();
                println!("播放已暂停！");
                let _ = self.app.emit_all(
                    "on-audio-thread-event",
                    AudioThreadEvent::PlayStatus {
                        is_playing: self.is_playing,
                    },
                );
                msg.ret(&self.app, None::<()>).unwrap();
            }
            AudioThreadMessage::PrevSong { .. } => {
                self.format_result = None;
                self.decoder = None;

                if self.playlist.len() > 2 {
                    if self.current_play_index == 1 {
                        self.current_play_index = self.playlist.len();
                    } else if self.current_play_index == 0 {
                        self.current_play_index = self.playlist.len() - 1;
                    } else {
                        self.current_play_index -= 2;
                    }
                }

                self.is_playing = true;
                self.player.stream().play().unwrap();
                println!("播放上一首歌曲！");
                msg.ret(&self.app, None::<()>).unwrap();
            }
            AudioThreadMessage::NextSong { .. } => {
                self.format_result = None;
                self.decoder = None;
                self.is_playing = true;
                self.player.stream().play().unwrap();
                println!("播放下一首歌曲！");
                msg.ret(&self.app, None::<()>).unwrap();
            }
            AudioThreadMessage::JumpToSong { song_index, .. } => {
                self.format_result = None;
                self.decoder = None;
                self.is_playing = true;
                if *song_index == 0 {
                    self.current_play_index = self.playlist.len();
                } else {
                    self.current_play_index = *song_index - 1;
                }
                self.player.stream().play().unwrap();
                println!("播放第 {} 首歌曲！", *song_index + 1);
                msg.ret(&self.app, None::<()>).unwrap();
            }
            AudioThreadMessage::SetPlaylist { songs, .. } => {
                self.playlist = songs.to_owned();
                println!("已设置播放列表，歌曲数量为 {}", songs.len());
                self.current_play_index = self.playlist.len();
                msg.ret(&self.app, None::<()>).unwrap();
            }
            AudioThreadMessage::SyncStatus => {
                let _ = self.app.emit_all(
                    "on-audio-thread-event",
                    AudioThreadEvent::SyncStatus {
                        ncm_id: self.current_song.ncm_id.to_owned(),
                        is_playing: self.is_playing,
                        duration: self.play_duration,
                        position: self.play_position,
                        load_position: self.load_position.load(std::sync::atomic::Ordering::SeqCst)
                            as f64
                            / u16::MAX as f64,
                        playlist: self.playlist.to_owned(),
                    },
                );
            }
            other => dbg!(other).ret(&self.app, None::<()>).unwrap(),
        }
    }

    pub fn process_audio(&mut self) {
        let mut is_song_finished = false;
        if let Some(format_result) = self.format_result.as_mut() {
            if !self.is_playing {
                return;
            }
            if let Some(decoder) = self.decoder.as_mut() {
                match format_result.format.next_packet() {
                    Ok(packet) => match decoder.decode(&packet) {
                        Ok(buf) => {
                            let time = self.timebase.calc_time(packet.ts);
                            self.play_position = time.seconds as f64 + time.frac;
                            let _ = self.app.emit_all(
                                "on-audio-thread-event",
                                AudioThreadEvent::PlayPosition {
                                    position: self.play_position,
                                },
                            );
                            self.player.write(buf);
                        }
                        Err(err) => {
                            println!("[WARN][AT] 解码器解码出错 {err}");
                        }
                    },
                    Err(DecodeError::IoError(err)) => match err.kind() {
                        ErrorKind::UnexpectedEof => {
                            if self
                                .full_downloaded_atom
                                .load(core::sync::atomic::Ordering::SeqCst)
                            {
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
                self.timebase = track.codec_params.time_base.unwrap_or_default();
                self.decoder = self
                    .codecs
                    .make(&track.codec_params, &Default::default())
                    .ok();
                let duration = self
                    .timebase
                    .calc_time(track.codec_params.n_frames.unwrap_or_default());
                self.play_duration = duration.seconds as f64 + duration.frac;
                let _ = self.app.emit_all(
                    "on-audio-thread-event",
                    AudioThreadEvent::LoadAudio {
                        ncm_id: self.current_song.ncm_id.to_owned(),
                        duration: self.play_duration,
                    },
                );
                let _ = self.app.emit_all(
                    "on-audio-thread-event",
                    AudioThreadEvent::PlayStatus {
                        is_playing: self.is_playing,
                    },
                );
            }
        } else {
            // 选择下一首歌
            if self.playlist.is_empty() {
                self.is_playing = false;
            } else {
                // 如果存在则中断正在流式播放的歌曲下载线程
                self.stop_download_atom
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                if let Some(handle) = self.download_thread_handle.take() {
                    let _ = handle.join();
                }
                self.stop_download_atom
                    .store(false, core::sync::atomic::Ordering::SeqCst);
                // 选歌
                self.current_play_index += 1;
                if self.current_play_index >= self.playlist.len() {
                    self.current_play_index = 0;
                }
                self.current_song = self.playlist[self.current_play_index].to_owned();
                println!(
                    "即将尝试播放下一首歌：{} ({})",
                    self.current_song.ncm_id, self.current_song.local_file
                );
                let _ = self.app.emit_all(
                    "on-audio-thread-event",
                    AudioThreadEvent::LoadingAudio {
                        ncm_id: self.current_song.ncm_id.to_owned(),
                    },
                );
                // 是否有本地文件
                if let Ok(file) = std::fs::OpenOptions::new()
                    .read(true)
                    .open(&self.current_song.local_file)
                {
                    self.full_downloaded_atom
                        .store(true, core::sync::atomic::Ordering::SeqCst);
                    let source_stream =
                        MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
                    self.format_result = self
                        .probe
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
                        self.current_song.ncm_id
                    );
                    let res = self
                        .session
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
                            .open(&self.audio_current_tmp_file)
                            .unwrap();
                        let mut output_file_reader = std::fs::OpenOptions::new()
                            .read(true)
                            .open(&self.audio_current_tmp_file)
                            .unwrap();
                        if let Ok(mut song_res) = self.session.get(song_url).send() {
                            let stop_downloaded_atom = self.stop_download_atom.clone();
                            let full_downloaded_atom = self.full_downloaded_atom.clone();
                            let _full_downloaded_atom = full_downloaded_atom.clone();
                            let download_size_atom = Arc::new(AtomicUsize::new(0));
                            let _download_size_atom = download_size_atom.clone();
                            let _app = self.app.clone();
                            self.download_thread_handle = Some(spawn(move || {
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
                                match self.probe.format(
                                    &Default::default(),
                                    source_stream,
                                    &Default::default(),
                                    &Default::default(),
                                ) {
                                    Ok(result) => {
                                        self.format_result = Some(result);
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
                        } else {
                            println!("下载失败，播放下一首歌");
                        }
                    }
                }
            }
        }
        if is_song_finished {
            self.format_result = None;
            self.decoder = None;
        }
    }
}
