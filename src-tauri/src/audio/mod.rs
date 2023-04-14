use std::time::Duration;

use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use std::thread::spawn;

use tauri::{Manager, State};

mod output;
mod player;
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
    LoadProgress { position: f64 },
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
        playlist: Vec<SongData>,
    },
    #[serde(rename_all = "camelCase")]
    PlayStatus { is_playing: bool },
    #[serde(rename_all = "camelCase")]
    LoadError { error: String },
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
pub fn send_msg_to_audio_thread(
    app_state: State<crate::AppState>,
    msg: AudioThreadMessage,
) -> std::result::Result<(), String> {
    if let AudioThreadMessage::SetCookie { cookie, .. } = &msg {
        *app_state.cookie.lock().unwrap() = cookie.to_owned();
        let mut session = app_state.session.lock().unwrap();
        session.header("cookie", cookie.to_owned());
    }
    send_msg_to_audio_thread_inner(msg)
}

pub fn audio_thread_main(app: tauri::AppHandle, rx: Receiver<AudioThreadMessage>) {
    println!("音频线程已开始运行！");
    let mut player = player::AudioPlayer::new(app);

    loop {
        if player.is_playing() {
            for msg in rx.try_iter() {
                player.process_message(msg);
            }
            player.process_audio();
        } else {
            match rx.recv() {
                Ok(msg) => player.process_message(msg),
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
