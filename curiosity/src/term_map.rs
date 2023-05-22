use std::{io::Cursor, ops::Deref};

use ph::fmph::Function as FPHash;
use smartstring::{Compact, SmartString};

pub struct SerializableFPHash(FPHash);

impl Deref for SerializableFPHash {
    type Target = FPHash;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl serde::Serialize for SerializableFPHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut b = Vec::with_capacity(self.0.write_bytes());
        self.0.write(&mut b).unwrap(); // will never fail, since b: Vec<u8>
        serializer.serialize_bytes(&b)
    }
}

impl<'de> serde::Deserialize<'de> for SerializableFPHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FPHashVisitor;
        impl<'de> serde::de::Visitor<'de> for FPHashVisitor {
            type Value = FPHash;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a bytes sequence representing an FPHash")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut c = Cursor::new(v);
                FPHash::read(&mut c).map_err(|e| E::custom(e.to_string()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de::Error;

                let mut bytes: Vec<u8> = Vec::with_capacity(seq.size_hint().unwrap_or(64));
                while let Some(b) = seq.next_element()? {
                    bytes.push(b);
                }

                let mut bytes_window = &bytes[..];

                FPHash::read(&mut bytes_window).map_err(|e| A::Error::custom(e.to_string()))
            }
        }

        Ok(SerializableFPHash(
            deserializer.deserialize_bytes(FPHashVisitor)?,
        ))
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TermMap {
    hasher: SerializableFPHash,
    keys: Vec<SmartString<Compact>>,
    values: Vec<u32>,
}

impl TermMap {
    pub fn construct(keys: Vec<String>, values: Vec<u32>) -> TermMap {
        assert!(keys.len() == values.len());
        let mut out_keys = vec![SmartString::<Compact>::new(); keys.len()];
        let mut out_vals = vec![0u32; values.len()];

        let hash_fn = FPHash::new(keys.clone());
        for (key, val) in keys.into_iter().zip(values.into_iter()) {
            let idx = hash_fn.get(&key).unwrap() as usize;
            out_keys[idx] = key.into();
            out_vals[idx] = val;
        }

        TermMap {
            hasher: SerializableFPHash(hash_fn),
            keys: out_keys,
            values: out_vals,
        }
    }

    pub fn get(&self, key: &str) -> Option<u32> {
        let idx = self.hasher.get(&key)? as usize;

        if self.keys[idx] != key {
            return None;
        }

        Some(self.values[idx])
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = (&SmartString<Compact>, &u32)> {
        self.keys.iter().zip(self.values.iter())
    }
}
