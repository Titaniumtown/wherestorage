use anyhow::{bail, Result};
pub struct ChunkBytes([u8; 9]);

// impl ChunkBytes {
//     fn to_bytes(self) -> [u8; 9] {
//         self.0
//     }
// }

pub struct ChunkJsonLimits {
    pub server_id_max: u16,
    pub server_id_min: u16,
    pub ping_max: u16,
    pub ping_min: u16,
    pub upload_max: u32,
    pub upload_min: u32,
    pub download_max: u32,
    pub download_min: u32,
}

const CHUNK_JSON_LIMITS: ChunkJsonLimits = ChunkJsonLimits {
    server_id_max: 11024, // 10bits // 2^10 -1 +server_id_min
    server_id_min: 10000,
    ping_max: 65535, // 16bits // 2^16 -1 +ping_min
    ping_min: 0,
    upload_max: 8388608, // 23bits // 2^23 -1 +upload_min
    upload_min: 1,
    download_max: 8388608, // 23bits // 2^23 -1 +download_min
    download_min: 1,
};

pub struct ChunkJson {
    pub server_id: u16, // 10000 to 65462          :::: 64689  states ::~15bits ::1.9 bytes::10bit ::1B + 7b
    pub ping: u16, // 0 to 65536              :::: 65537  states ::~16bits ::2 bytes  ::16bit ::2B
    pub upload: u32, // 1 to 9999999 (7 of them):::: 9999999states ::~23bits ::2.9 bytes::23bit ::2B + 7b
    pub download: u32, // 1 to 9999999 (7 of them):::: 9999999states ::~23bits ::2.9 bytes::23bit ::2B + 7b
} // total log_2(9999999*9999999*65537*64957) = ~78 bits = ~9 bytes = 9*8 = 72 bits

impl std::fmt::Display for ChunkJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{\"serverid\":{},\"ping\":{},\"upload\":{},\"download\":{},\"hash\":\"{:x}\"}}",
            self.server_id,
            self.ping,
            self.upload,
            self.download,
            md5::compute(format!(
                "{}-{}-{}-817d699764d33f89c",
                self.ping, self.upload, self.download
            ))
        )
    }
}

impl ChunkJson {
    pub fn is_bytable(self) -> Result<Self> {
        if !(CHUNK_JSON_LIMITS.server_id_min..=CHUNK_JSON_LIMITS.server_id_max)
            .contains(&self.server_id)
        {
            bail!("'serverid' is not in range.")
        } else if !(CHUNK_JSON_LIMITS.ping_min..=CHUNK_JSON_LIMITS.ping_max).contains(&self.ping) {
            bail!("'ping' is not in range.")
        } else if !(CHUNK_JSON_LIMITS.upload_min..=CHUNK_JSON_LIMITS.upload_max)
            .contains(&self.upload)
        {
            bail!("'upload' is not in range.")
        } else if !(CHUNK_JSON_LIMITS.download_min..=CHUNK_JSON_LIMITS.download_max)
            .contains(&self.download)
        {
            bail!("'download' is not in range")
        } else {
            Ok(self)
        }
    }
}

impl From<ChunkBytes> for ChunkJson {
    /* byte allocation to field
       bytes[0]            -> server_id
       bytes[1,2]          -> ping
       bytes[3,4]          -> upload
       bytes[5,6]          -> download
       bytes[7] & 0b1      -> server_id
       bytes[7] >> 1       -> upload
       bytes[8] & 0b1      -> download
       bytes[8] >> 1       -> download
    */
    fn from(cb: ChunkBytes) -> Self {
        let server_id: u16 = CHUNK_JSON_LIMITS.server_id_min
            + (((cb.0[0] as u16) << 2) & 0b11111111_00u16)
            + ((((cb.0[7] as u16) & 0b1) << 1) & 0b1_0u16)
            + ((cb.0[8] as u16) & 0b1);
        let ping: u16 = CHUNK_JSON_LIMITS.ping_min
            + (((cb.0[1] as u16) << 8) & 0b11111111_00000000u16)
            + ((cb.0[2] as u16) & 0b11111111u16);
        let upload: u32 = CHUNK_JSON_LIMITS.upload_min
            + (((cb.0[3] as u32) << (8 + 7)) & 0b11111111_00000000_0000000u32)
            + (((cb.0[4] as u32) << 7) & 0b11111111_0000000u32)
            + (((cb.0[7] as u32) >> 1) & 0b1111111u32);
        let download: u32 = CHUNK_JSON_LIMITS.upload_min
            + (((cb.0[5] as u32) << (8 + 7)) & 0b11111111_00000000_0000000u32)
            + (((cb.0[6] as u32) << 7) & 0b11111111_0000000u32)
            + (((cb.0[8] as u32) >> 1) & 0b1111111u32);

        Self {
            server_id,
            ping,
            upload,
            download,
        }
    }
}

impl TryFrom<ChunkJson> for ChunkBytes {
    type Error = anyhow::Error;

    fn try_from(cj: ChunkJson) -> Result<Self> {
        let cjb = cj.is_bytable()?;
        let mut bytes: [u8; 9] = [0; 9];

        let mut server_id = cjb.server_id - CHUNK_JSON_LIMITS.server_id_min;
        bytes[8] += (server_id & 0b1).to_le_bytes()[0];
        server_id = (server_id >> 1) & 0b11111111_1u16;
        bytes[7] += (server_id & 0b1).to_le_bytes()[0];
        server_id = (server_id >> 1) & 0b11111111u16;
        bytes[0] += server_id.to_le_bytes()[0];

        let ping = (cjb.ping - CHUNK_JSON_LIMITS.ping_min).to_le_bytes();
        bytes[2] += ping[0];
        bytes[1] += ping[1];

        let upload = cjb.upload - CHUNK_JSON_LIMITS.upload_min;
        bytes[7] += ((upload << 1) & 0b11111110u32).to_le_bytes()[0];
        let upload_bytes = ((upload >> 7) & 0b11111111_11111111u32).to_le_bytes();
        bytes[4] += upload_bytes[0];
        bytes[3] += upload_bytes[1];

        let download = cjb.download - CHUNK_JSON_LIMITS.download_min;
        bytes[8] += ((download << 1) & 0b11111110u32).to_le_bytes()[0];
        let download_bytes = ((download >> 7) & 0b11111111_11111111u32).to_le_bytes();
        bytes[6] += download_bytes[0];
        bytes[5] += download_bytes[1];

        Ok(ChunkBytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
    use std::fmt::{self, Formatter};

    impl Debug for ChunkJson {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.debug_struct("ChunkJson")
                .field("server_id", &self.server_id)
                .field("ping", &self.ping)
                .field("upload", &self.upload)
                .field("download", &self.download)
                .finish()
        }
    }

    #[test]
    fn max() {
        let a = ChunkJson::from(ChunkBytes([u8::MAX; 9]));
        dbg!(&a);
        assert_eq!(a.upload, 2u32.pow(23) - 1 + 1);
        assert_eq!(a.upload, a.download);
    }

    #[test]
    fn min() {
        let b = ChunkJson::from(ChunkBytes([u8::MIN; 9]));
        dbg!(&b);
        assert_eq!(b.upload, 1);
        assert_eq!(b.upload, b.download);
    }

    #[test]
    fn good_enc_dec() {
        let b: [u8; 9] = [100, 101, 102, 103, 104, 105, 106, 107, 108];
        let cb = ChunkBytes(b);
        let cj = ChunkJson::from(cb);
        assert_eq!(ChunkBytes::try_from(cj).unwrap().0, b);
    }
}
