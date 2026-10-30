use crate::keys::KeyCombo;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S: Serializer>(value: &KeyCombo, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_shortcut_string().unwrap_or_default())
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<KeyCombo, D::Error> {
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(KeyCombo::from_shortcut_string(&value.unwrap_or_default()))
}
