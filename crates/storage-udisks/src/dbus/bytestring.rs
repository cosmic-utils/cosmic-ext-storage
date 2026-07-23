use zbus::zvariant::{OwnedValue, Value};

pub fn encode_bytestring(value: &str) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    bytes
}

pub fn bytestring_owned_value(value: &str) -> OwnedValue {
    Value::from(encode_bytestring(value))
        .try_into()
        .expect("zvariant Value<Vec<u8>> should convert into OwnedValue")
}

pub fn owned_value_to_bytestring(value: &OwnedValue) -> Option<String> {
    let bytes: Vec<u8> = value.clone().try_into().ok()?;
    Some(decode_c_string_bytes(&bytes))
}

pub fn decode_c_string_bytes(bytes: &[u8]) -> String {
    let raw = match bytes.split(|b| *b == 0).next() {
        Some(v) => v,
        None => bytes,
    };

    String::from_utf8_lossy(raw).to_string()
}

pub fn decode_mount_points(mount_points: Vec<Vec<u8>>) -> Vec<String> {
    mount_points
        .into_iter()
        .filter_map(|mp| {
            let decoded = decode_c_string_bytes(&mp);
            if decoded.is_empty() {
                None
            } else {
                Some(decoded)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_c_string_bytes_truncates_nul() {
        let bytes = b"/run/media/user/DISK\0garbage";
        assert_eq!(decode_c_string_bytes(bytes), "/run/media/user/DISK");
    }

    #[test]
    fn decode_mount_points_filters_empty_entries() {
        let decoded = decode_mount_points(vec![
            b"/mnt/a\0".to_vec(),
            b"\0".to_vec(),
            Vec::new(),
            b"/mnt/b".to_vec(),
        ]);

        assert_eq!(decoded, vec!["/mnt/a".to_string(), "/mnt/b".to_string()]);
    }
}
