pub mod h3edgemap {
    /// serializers to serialize/deserialize edgemaps faster
    ///
    /// Differences:
    /// * H3Edges are serialized as u64. This is much faster during deserialization. Not sure why.
    /// * SizeHints are not cut-off as serdes default deserializers do (for security reasons, at 4096 elements)
    ///   This results in fewer reallocations of the map
    use std::fmt;
    use std::marker::PhantomData;
    use std::result::Result;

    use serde::de::{MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::collections::H3EdgeMap;
    use crate::h3ron::Index;

    struct CapacityMapVisitor<V> {
        marker: PhantomData<fn() -> H3EdgeMap<V>>,
    }

    impl<V> CapacityMapVisitor<V> {
        fn new() -> Self {
            Self {
                marker: PhantomData,
            }
        }
    }

    impl<'de, V> Visitor<'de> for CapacityMapVisitor<V>
    where
        V: Deserialize<'de>,
    {
        type Value = H3EdgeMap<V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("edge map failed")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, <M as MapAccess<'de>>::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = H3EdgeMap::default();
            map.reserve(
                access
                    .size_hint()
                    .unwrap_or(4096)
                    .saturating_sub(map.capacity()),
            );
            while let Some((k, v)) = access.next_entry::<u64, V>()? {
                map.insert(h3ron::H3Edge::new(k), v);
            }
            Ok(map)
        }
    }
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<H3EdgeMap<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        deserializer.deserialize_map(CapacityMapVisitor::new())
    }

    pub fn serialize<S, T>(edge_map: &H3EdgeMap<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        let mut map = serializer.serialize_map(Some(edge_map.len()))?;
        for (k, v) in edge_map {
            map.serialize_entry(&(k.h3index() as u64), v)?;
        }
        map.end()
    }
}
