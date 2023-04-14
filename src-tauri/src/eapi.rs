use anyhow::Context;
use libaes::Cipher;
use once_cell::unsync::Lazy;
use tauri::State;

use crate::AppState;

const EAPI_KEY: &[u8; 16] = b"e82ckenh8dichen8";
std::thread_local! {
    static EAPI_CIPHER: Lazy<Cipher> = Lazy::new(|| Cipher::new_128(EAPI_KEY));
}

pub fn eapi_encrypt(data: &str) -> Vec<u8> {
    EAPI_CIPHER.with(|c| c.ebc_encrypt(data.as_bytes()))
}

#[tauri::command]
pub fn tauri_eapi_encrypt(data: &str) -> String {
    faster_hex::hex_string(&eapi_encrypt(data))
}

#[tauri::command]
pub fn tauri_eapi_encrypt_for_request(url: &str, data: &str) -> String {
    eapi_encrypt_for_request(url, data)
}

pub fn eapi_encrypt_for_request(url: &str, data: &str) -> String {
    EAPI_CIPHER.with(|c| {
        let msg = concat_string::concat_string!("nobody", url, "use", data, "md5forencrypt");
        let hash = md5::compute(msg);
        let data = concat_string::concat_string!(
            url,
            "-36cd479b6b5-",
            data,
            "-36cd479b6b5-",
            faster_hex::hex_string(hash.as_slice())
        );
        faster_hex::hex_string(&c.ebc_encrypt(data.as_bytes()))
    })
}

#[tauri::command]
pub fn tauri_eapi_decrypt(data: &str) -> Result<String, String> {
    if data.is_empty() {
        return Ok("".into());
    }
    let mut buf = vec![0; data.len() / 2];
    faster_hex::hex_decode(data.as_bytes(), &mut buf).map_err(|x| x.to_string())?;
    Ok(eapi_decrypt(&buf))
}

pub fn eapi_decrypt(data: &[u8]) -> String {
    EAPI_CIPHER.with(|c| String::from_utf8_lossy(&c.ebc_decrypt(data)).to_string())
}

pub async fn eapi_request(
    app_state: State<'_, AppState>,
    url: String,
    data: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let url = url.parse::<tauri::http::Uri>().context("请求链接不合法")?;
    let should_encrypt = url.path().starts_with("/eapi");
    let data = if should_encrypt {
        eapi_encrypt_for_request(
            &concat_string::concat_string!("/api", url.path().trim_start_matches("/eapi")),
            serde_json::to_string(&data)
                .context("无法序列化提交数据")?
                .as_str(),
        )
        .as_bytes()
        .to_vec()
    } else {
        serde_json::to_vec(&data).context("无法序列化提交数据")?
    };
    let req = app_state
        .session
        .lock()
        .unwrap()
        .post(url.to_string())
        .bytes(data)
        .header(
            "content-type",
            if should_encrypt {
                "application/x-www-form-urlencoded"
            } else {
                "application/json"
            },
        );
    let res = tauri::async_runtime::spawn_blocking(move || req.send().map(|x| x.bytes()))
        .await
        .context("响应线程执行出错")?
        .context("无法发送请求")?
        .context("响应接收失败")?;
    if let Some(b) = res.first().copied() {
        if b == b'{' {
            if let Ok(obj) = serde_json::from_slice(&res) {
                return Ok(obj);
            }
        }
    }
    let res = eapi_decrypt(&res);

    serde_json::from_str(&res).context("无法解析响应数据")
}

#[tauri::command]
pub async fn tauri_eapi_request(
    app_state: State<'_, AppState>,
    url: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    eapi_request(app_state, url, data)
        .await
        .map_err(|x| x.to_string())
}
