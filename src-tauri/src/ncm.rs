use anyhow::Context;
use base64::prelude::*;
use byteorder::*;
use libaes::*;
use std::io::{Read, Seek};

/// 一个支持流式读取的 NCM 格式结构
pub struct NCMFile<R> {
    inner: R,
    data_pos: u64,
    rc4: crate::rc4::RC4,
}

const CORE_KEY: [u8; 16] = [
    0x68, 0x7A, 0x48, 0x52, 0x41, 0x6D, 0x73, 0x6F, 0x35, 0x6B, 0x49, 0x6E, 0x62, 0x61, 0x78, 0x57,
];
const META_KEY: [u8; 16] = [
    0x23, 0x31, 0x34, 0x6C, 0x6A, 0x6B, 0x5F, 0x21, 0x5C, 0x5D, 0x26, 0x30, 0x55, 0x3C, 0x27, 0x28,
];

impl<R: Read + Seek> NCMFile<R> {
    pub fn new(mut reader: R) -> anyhow::Result<Self> {
        let mut magic_header = [0u8; 10];

        reader
            .read_exact(&mut magic_header)
            .context("无法读取文件魔法头")?;

        anyhow::ensure!(
            magic_header == [0x43, 0x54, 0x45, 0x4E, 0x46, 0x44, 0x41, 0x4D, 0x01, 0x70],
            "文件头格式错误"
        );

        let key_length = dbg!(reader.read_u32::<LE>()?) as usize;
        let mut rc4_key = vec![0u8; key_length];

        reader
            .read_exact(&mut rc4_key)
            .context("无法读取 RC4 密钥")?;

        rc4_key.iter_mut().for_each(|x| {
            *x ^= 0x64;
        });

        let cipher = Cipher::new_128(&CORE_KEY);
        let mut rc4_key = cipher.ebc_decrypt(&rc4_key);
        rc4_key.splice(0..17, []);

        let meta_data_length = dbg!(reader.read_u32::<LE>()?) as usize;

        let mut meta_data = vec![0u8; meta_data_length];
        reader.read_exact(&mut meta_data)?;
        meta_data.iter_mut().for_each(|x| {
            *x ^= 0x63;
        });

        let cipher = Cipher::new_128(&META_KEY);
        let meta_data = cipher.ebc_decrypt(&BASE64_STANDARD.decode(&meta_data[22..])?);

        dbg!(String::from_utf8_lossy(&meta_data[6..]));

        let _crc = reader.read_u32::<LE>()?;
        reader.seek(std::io::SeekFrom::Current(5))?;
        let album_image_size = dbg!(reader.read_u32::<LE>()? as i64);
        // 现在的网易云已不再内嵌专辑图片，干脆直接跳过读取图片吧
        reader.seek(std::io::SeekFrom::Current(album_image_size))?;

        // 记录当前真正的音频数据的开始位置
        let data_pos = reader.stream_position()?;

        Ok(Self {
            inner: reader,
            data_pos,
            rc4: crate::rc4::RC4::new(&rc4_key),
        })
    }
}

impl<R: Read> Read for NCMFile<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.inner.read(buf) {
            Ok(read_len) => {
                self.rc4.prga(&mut buf[..read_len]);
                Ok(read_len)
            }
            Err(err) => Err(err),
        }
    }
}

impl<R: Seek> Seek for NCMFile<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Start(offset) => self
                .inner
                .seek(std::io::SeekFrom::Start(offset + self.data_pos)),
            std::io::SeekFrom::End(offset) => self
                .inner
                .seek(std::io::SeekFrom::End(offset))
                .map(|x| x - self.data_pos),
            std::io::SeekFrom::Current(offset) => {
                let cur_pos = self.inner.stream_position()? as i64;
                let target_pos = (cur_pos + offset).max(self.data_pos as i64);
                self.inner
                    .seek(std::io::SeekFrom::Current(target_pos - cur_pos))
            }
        }
        .map(|x| x - self.data_pos)
    }
}
