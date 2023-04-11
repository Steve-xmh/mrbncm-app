use std::time::Duration;

use once_cell::sync::Lazy;
use tauri::{
    async_runtime::{channel, spawn, Mutex, Sender},
    Manager,
};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SongData {
    pub ncm_id: String,
    pub local_file: String,
    pub orig_order: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum AudioThreadMessage {
    ResumeAudio {
        callback_id: String,
    },
    PauseAudio {
        callback_id: String,
    },
    SeekAudio {
        callback_id: String,
        position: Duration,
    },
    JumpToSong {
        callback_id: String,
        song_index: usize,
    },
    PrevSong {
        callback_id: String,
    },
    NextSong {
        callback_id: String,
    },
    SetPlaylist {
        callback_id: String,
        songs: Vec<SongData>,
    },
    SetCookie {
        callback_id: String,
        cookie: String,
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
        app: tauri::AppHandle,
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

static MSG_SENDER: Lazy<Mutex<Option<Sender<AudioThreadMessage>>>> = Lazy::new(|| Mutex::new(None));

#[tauri::command]
pub async fn send_msg_to_audio_thread(msg: AudioThreadMessage) {
    if let Some(sx) = MSG_SENDER.blocking_lock().as_ref() {
        sx.blocking_send(msg).unwrap();
    }
}

#[tauri::command]
pub async fn init_audio_thread(app: tauri::AppHandle) -> Result<(), String> {
    if MSG_SENDER.lock().await.is_none() {
        let (sx, mut rx) = channel::<AudioThreadMessage>(64);
        (*MSG_SENDER.lock().await) = Some(sx);
        spawn(async move { while let Some(msg) = rx.recv().await {} });
    }
    Ok(())
}
