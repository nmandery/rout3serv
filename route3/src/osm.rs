use crate::constants::Weight;
use route3_core::osm::EdgeProperties;
use route3_core::osmpbfreader::Tags;

pub fn way_properties(tags: &Tags) -> Option<EdgeProperties<Weight>> {
    // https://wiki.openstreetmap.org/wiki/Key:highway
    if let Some(highway_value) = tags.get("highway") {
        match highway_value.to_lowercase().as_str() {
            "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary" | "primary_link" => {
                Some(Weight::from(3.0))
            }
            "secondary" | "secondary_link" => Some(Weight::from(4.0)),
            "tertiary" | "tertiary_link" => Some(Weight::from(5.0)),
            "unclassified" | "residential" | "living_street" => Some(Weight::from(8.0)),
            "road" => Some(Weight::from(9.0)),
            //"service" | "track" => Some(WeightType::from(20.0)),
            _ => None,
        }
        .map(|weight| {
            // oneway streets (https://wiki.openstreetmap.org/wiki/Key:oneway)
            // NOTE: reversed direction "oneway=-1" is not supported
            let is_bidirectional = tags
                .get("oneway")
                .map(|v| v.to_lowercase() != "yes")
                .unwrap_or(true);

            EdgeProperties {
                is_bidirectional,
                weight,
            }
        })
    } else {
        None
    }
}
