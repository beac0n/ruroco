use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserializer, Serializer};
use std::collections::HashMap;
use std::fmt;

struct MapVisitor;

impl<'de> Visitor<'de> for MapVisitor {
    type Value = HashMap<u64, u128>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a map of u64 to string-encoded u128")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));
        while let Some((k, v)) = access.next_entry::<u64, String>()? {
            let v = v.parse::<u128>().map_err(serde::de::Error::custom)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

pub(crate) fn serialize<S>(map: &HashMap<u64, u128>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut m = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        m.serialize_entry(k, &v.to_string())?;
    }
    m.end()
}

pub(crate) fn deserialize<'d, D>(deserializer: D) -> Result<HashMap<u64, u128>, D::Error>
where
    D: Deserializer<'d>,
{
    deserializer.deserialize_map(MapVisitor)
}

#[cfg(test)]
mod tests {
    use crate::server::blocklist::Blocklist;

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut blocklist = Blocklist::create(dir.path()).unwrap();
        blocklist.add([1, 2, 3, 4, 5, 6, 7, 8], 123456789012345678901234567890);
        blocklist.add([8, 7, 6, 5, 4, 3, 2, 1], u128::MAX);
        blocklist.save().unwrap();

        let loaded = Blocklist::create(dir.path()).unwrap();
        assert_eq!(loaded.get().len(), 2);
        assert!(loaded.is_blocked([1, 2, 3, 4, 5, 6, 7, 8], 123456789012345678901234567890));
        assert!(loaded.is_blocked([8, 7, 6, 5, 4, 3, 2, 1], u128::MAX));
    }
}
