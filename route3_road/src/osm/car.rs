use h3ron_graph::formats::osm::osmpbfreader::Tags;
use h3ron_graph::formats::osm::EdgeProperties;
use uom::si::f32::Length;

use crate::osm::infer_max_speed;
use crate::weight::RoadWeight;

pub fn way_properties(tags: &Tags, edge_length: Length) -> Option<EdgeProperties<RoadWeight>> {
    // https://wiki.openstreetmap.org/wiki/Key:highway or https://wiki.openstreetmap.org/wiki/DE:Key:highway
    // TODO: make use of `access` tag: https://wiki.openstreetmap.org/wiki/Key:access
    if let Some(highway_value) = tags.get("highway") {
        let highway_class = highway_value.to_lowercase();
        match highway_class.as_str() {
            "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary" | "primary_link" => {
                Some(3.0)
            }
            "secondary" | "secondary_link" => Some(4.0),
            "tertiary" | "tertiary_link" => Some(5.0),
            "unclassified" | "residential" | "living_street" | "service" | "rural" => Some(8.0),
            "road" => Some(9.0),
            // "track" => Some(200.0), // mostly non-public agriculture/forestry roads
            "pedestrian" | "footway" => Some(50.0), // fussgängerzone
            _ => None,
        }
        .map(|category_weight| {
            // oneway streets (https://wiki.openstreetmap.org/wiki/Key:oneway)
            // NOTE: reversed direction "oneway=-1" is not supported
            let is_bidirectional = tags
                .get("oneway")
                .map(|v| v.to_lowercase() != "yes")
                .unwrap_or(true);

            let max_speed = infer_max_speed(tags, &highway_class);

            EdgeProperties {
                is_bidirectional,
                weight: RoadWeight::new(category_weight, edge_length / max_speed),
            }
        })
    } else {
        None
    }
}
