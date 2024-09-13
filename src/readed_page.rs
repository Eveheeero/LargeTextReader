use serde::{ser::SerializeMap, Deserialize, Serialize};

pub(super) struct ReadedPage {
    pub(super) page: usize,
    pub(super) readed_time: time::OffsetDateTime,
}

impl Default for ReadedPage {
    fn default() -> Self {
        Self {
            page: 0,
            readed_time: time::OffsetDateTime::now_utc().replace_offset(time::macros::offset!(+9)),
        }
    }
}

impl ReadedPage {
    pub(super) fn new(page: usize) -> Self {
        Self {
            page,
            ..Default::default()
        }
    }
}

impl Serialize for ReadedPage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("page", &self.page)?;
        state.serialize_entry("readed_time", &self.readed_time)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ReadedPage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ReadedPageVisitor;

        impl<'de> serde::de::Visitor<'de> for ReadedPageVisitor {
            type Value = ReadedPage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a ReadedPage")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut page = None;
                let mut readed_time = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        "page" => {
                            if page.is_some() {
                                return Err(serde::de::Error::duplicate_field("page"));
                            }
                            page = Some(map.next_value()?);
                        }
                        "readed_time" => {
                            if readed_time.is_some() {
                                return Err(serde::de::Error::duplicate_field("readed_time"));
                            }
                            readed_time = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                key,
                                &["page", "readed_time"],
                            ))
                        }
                    }
                }
                let page = page.ok_or_else(|| serde::de::Error::missing_field("page"))?;
                let readed_time =
                    readed_time.ok_or_else(|| serde::de::Error::missing_field("readed_time"))?;
                Ok(ReadedPage { page, readed_time })
            }
        }

        deserializer.deserialize_map(ReadedPageVisitor)
    }
}
