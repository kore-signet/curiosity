use ph::FPHash;
use smartstring::{Compact, SmartString};

pub struct TermMap {
    hasher: ph::FPHash,
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
            hasher: hash_fn,
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
    pub fn iter(&self) -> impl Iterator<Item = (&SmartString<Compact>, &u32)> {
        self.keys.iter().zip(self.values.iter())
    }
}
