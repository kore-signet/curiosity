use serde::de::IntoDeserializer;

use smallvec::SmallVec;

use serde::Deserialize;

pub fn page_size_default() -> usize {
    50
}

// i love that serde makes me write these. i love it actually
pub fn is_false(b: &bool) -> bool {
    *b
}

// https://github.com/actix/actix-web/issues/1301#issuecomment-747403932
pub fn deserialize_stringified_list<'de, D, I>(
    deserializer: D,
) -> std::result::Result<SmallVec<[I; 16]>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    I: serde::de::DeserializeOwned,
{
    struct StringVecVisitor<I>(std::marker::PhantomData<I>);

    impl<'de, I> serde::de::Visitor<'de> for StringVecVisitor<I>
    where
        I: serde::de::DeserializeOwned,
    {
        type Value = SmallVec<[I; 16]>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string containing a list")
        }

        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                return Ok(SmallVec::new());
            }

            let mut ids = SmallVec::new();
            for id in v.split(',') {
                let id = I::deserialize(id.into_deserializer())?;
                ids.push(id);
            }
            Ok(ids)
        }
    }

    if deserializer.is_human_readable() {
        deserializer.deserialize_any(StringVecVisitor(std::marker::PhantomData::<I>))
    } else {
        SmallVec::<[I; 16]>::deserialize(deserializer)
    }
}
