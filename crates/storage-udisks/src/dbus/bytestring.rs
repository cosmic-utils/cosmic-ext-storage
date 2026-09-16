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

#[path = "../../tests/unit/dbus/bytestring_tests.rs"]
#[cfg(test)]
mod tests;
