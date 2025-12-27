use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::{Deserialize, Deserializer, Serializer};

/// 序列化字节为 base64 字符串。
pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded = STANDARD.encode(bytes);
    serializer.serialize_str(&encoded)
}

/// 反序列化 base64 字符串为字节。
pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    STANDARD
        .decode(encoded.as_bytes())
        .map_err(serde::de::Error::custom)
}

pub mod option {
    use super::*;
    use serde::de::Error as _;

    /// 序列化 Option<Vec<u8>> 为 base64 字符串。
    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(bytes) => serializer.serialize_some(&STANDARD.encode(bytes)),
            None => serializer.serialize_none(),
        }
    }

    /// 反序列化 base64 字符串为 Option<Vec<u8>>。
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = Option::<String>::deserialize(deserializer)?;
        match encoded {
            Some(value) => STANDARD
                .decode(value.as_bytes())
                .map(Some)
                .map_err(D::Error::custom),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_base64() {
        let input = b"hello";
        let encoded = STANDARD.encode(input);
        let decoded = STANDARD.decode(encoded.as_bytes()).unwrap();
        assert_eq!(input.to_vec(), decoded);
    }
}
