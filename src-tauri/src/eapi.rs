use concat_string::concat_string;
use libaes::Cipher;
use once_cell::unsync::Lazy;

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
