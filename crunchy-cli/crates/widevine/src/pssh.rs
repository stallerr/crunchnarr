use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::Error;

/// PSSH (Protection System Specific Header)
///
/// The PSSH object is used to identify the protected medium.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
#[non_exhaustive]
pub struct Pssh {
    pub version: u8,
    pub flags: u32,
    pub init_data: Vec<u8>,
    pub key_ids: Vec<[u8; 16]>,
}

/// Widevine DRM System ID: `edef8ba979d6-4acea3-c827dcd51d21ed`
const WIDEVINE_SYSTEM_ID: [u8; 16] = [
    0xed, 0xef, 0x8b, 0xa9, 0x79, 0xd6, 0x4a, 0xce, 0xa3, 0xc8, 0x27, 0xdc, 0xd5, 0x1d, 0x21, 0xed,
];

impl Pssh {
    /// Parse base64-formatted PSSH data
    pub fn from_b64(pssh: &str) -> Result<Self, Error> {
        log::trace!("Decoding base64 PSSH ({} chars)", pssh.len());
        let pssh_bts = data_encoding::BASE64
            .decode(pssh.as_bytes())
            .map_err(|e| Error::InvalidInput(format!("base64: {e}").into()))?;
        Self::from_bytes(&pssh_bts)
    }

    /// Create a new PSSH object from either a MP4 PSSH box or a WidevineCencHeader
    pub fn from_bytes(pssh: &[u8]) -> Result<Self, Error> {
        log::debug!("Parsing PSSH box ({} bytes)", pssh.len());
        let mut rdr = Cursor::new(pssh);
        let size = rdr.read_u32::<BigEndian>()?;
        if pssh.len() != size as usize {
            return Err(Error::InvalidInput("unexpected length".into()));
        }

        let mut box_header = [0u8; 4];
        rdr.read_exact(&mut box_header)?;
        if &box_header != b"pssh" {
            return Err(Error::InvalidInput("no pssh header".into()));
        }

        Self::from_box_content(&mut rdr)
    }

    /// Create a new PSSH object from the MP4 PSSH box content (without size field and "pssh" header name)
    pub fn from_box_content<T: Read>(rdr: &mut T) -> Result<Self, Error> {
        let version_and_flags = rdr.read_u32::<BigEndian>()?;
        let version: u8 = (version_and_flags >> 24) as u8;
        let flags = version_and_flags & 0xffffff;
        log::debug!("PSSH version: {}, flags: {:#x}", version, flags);
        if version > 1 {
            return Err(Error::InvalidInput(
                format!("unsupported PSSH version {version}").into(),
            ));
        }

        let mut system_id = [0u8; 16];
        rdr.read_exact(&mut system_id)?;
        log::trace!("System ID: {}", data_encoding::HEXLOWER.encode(&system_id));
        if system_id != WIDEVINE_SYSTEM_ID {
            return Err(Error::InvalidInput(
                format!(
                    "unsupported DRM system ID: {}",
                    data_encoding::HEXLOWER.encode(&system_id)
                )
                .into(),
            ));
        }

        let mut key_ids = Vec::new();
        if version == 1 {
            let kid_count = rdr.read_u32::<BigEndian>()?;
            for _ in 0..kid_count {
                let mut key_id = [0u8; 16];
                rdr.read_exact(&mut key_id)?;
                key_ids.push(key_id);
            }
        }

        let init_data_len = rdr.read_u32::<BigEndian>()?;
        let mut init_data = vec![0; init_data_len as usize];
        rdr.read_exact(&mut init_data)?;

        log::debug!(
            "Parsed PSSH: {} key IDs, {} bytes init_data",
            key_ids.len(),
            init_data.len()
        );

        Ok(Self {
            version,
            flags,
            init_data,
            key_ids,
        })
    }

    /// Try to extract a PSSH object from the header of a MP4 file
    pub fn from_mp4_file<T: Read>(rdr: &mut T) -> Result<Self, Error> {
        crate::mp4::extract_pssh_from_mp4(rdr).map_err(Error::from)
    }

    /// Convert the PSSH into a binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let size = self.len();
        let mut res = Vec::with_capacity(size);
        _ = res.write_u32::<BigEndian>(size.try_into().expect("pssh too big"));
        res.extend_from_slice(b"pssh");
        _ = res.write_u32::<BigEndian>(((self.version as u32) << 24) | (self.flags & 0xffffff));
        res.extend_from_slice(&WIDEVINE_SYSTEM_ID);
        if self.version == 1 {
            _ = res
                .write_u32::<BigEndian>(self.key_ids.len().try_into().expect("key id len too big"));
            for kid in &self.key_ids {
                res.extend_from_slice(kid);
            }
        }
        res.extend_from_slice(&self.init_data);
        res
    }

    /// Convert the PSSH into the Base64 format
    pub fn to_b64(&self) -> String {
        data_encoding::BASE64.encode(&self.to_bytes())
    }

    /// Get the size of the PSSH object in bytes
    fn len(&self) -> usize {
        let mut size = self.init_data.len() + 28;
        if self.version == 1 {
            size += 4 + self.key_ids.len() * 16;
        }
        size
    }
}

impl TryFrom<&[u8]> for Pssh {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(value)
    }
}

impl std::fmt::Display for Pssh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        data_encoding::BASE64.encode_write(&self.to_bytes(), f)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader};

    use path_macro::path;

    use super::*;

    #[test]
    fn parse_pssh() {
        let pssh =
        Pssh::from_b64("AAAAW3Bzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAADsIARIQ62dqu8s0Xpa7z2FmMPGj2hoNd2lkZXZpbmVfdGVzdCIQZmtqM2xqYVNkZmFsa3IzaioCSEQyAA==").unwrap();
        assert_eq!(
            pssh.init_data,
            [
                8, 1, 18, 16, 235, 103, 106, 187, 203, 52, 94, 150, 187, 207, 97, 102, 48, 241,
                163, 218, 26, 13, 119, 105, 100, 101, 118, 105, 110, 101, 95, 116, 101, 115, 116,
                34, 16, 102, 107, 106, 51, 108, 106, 97, 83, 100, 102, 97, 108, 107, 114, 51, 106,
                42, 2, 72, 68, 50, 0,
            ]
        );
        assert!(pssh.key_ids.is_empty());
    }

    #[test]
    fn parse_mp4() {
        let path = path!(env!("CARGO_MANIFEST_DIR") / "testfiles" / "test.mp4.head");

        let mut reader = BufReader::new(File::open(&path).unwrap()).take(1000);
        let pssh = Pssh::from_mp4_file(&mut reader).unwrap();
        assert_eq!(
            pssh.to_b64(),
            "AAAANHBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7SIQaKNPO2HDSnOwBvdjjQvBTkjj3JWbBg=="
        );
    }
}
