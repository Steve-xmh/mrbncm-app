pub struct RC4 {
    ksa_box: [u8; 256],
}

impl RC4 {
    pub fn new(key: &[u8]) -> Self {
        Self {
            ksa_box: gen_ksa(key),
        }
    }

    /// 随机数数据加解密
    ///
    /// 如果传入数据是密文，则会被解密，如果是原文则会被加密。
    pub fn prga(&self, data: &mut [u8]) {
        let mut i = 0;
        let mut j = 0;
        (0..data.len()).for_each(|k| {
            i = (k + 1) & 0xFF;
            j = (self.ksa_box[i] as usize + i) & 0xFF;
            data[k] ^= self.ksa_box[(self.ksa_box[i] as usize + self.ksa_box[j] as usize) & 0xFF];
        });
    }
}

fn gen_ksa(key: &[u8]) -> [u8; 256] {
    debug_assert!(
        !key.is_empty() && key.len() <= 256,
        "Key length must be in 0-256"
    );
    let mut ksa = [0u8; 256];
    (0..256).for_each(|i| {
        ksa[i] = i as u8;
    });
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + ksa[i] as usize + key[i % key.len()] as usize) & 0xFF;
        ksa.swap(i, j);
    }
    ksa
}
