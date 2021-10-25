use h3ron::algorithm::cell_centroid_distance_m;
use h3ron::H3Edge;
use h3ron_graph::formats::osm::osmpbfreader::Tags;
use h3ron_graph::formats::osm::{EdgeProperties, WayAnalyzer};
use uom::si::f32::{Length, Velocity};
use uom::si::length::meter;

use crate::osm::infer_max_speed;
use crate::weight::RoadWeight;

pub struct CarWayProperties {
    max_speed: Velocity,
    category_weight: f32,
    is_bidirectional: bool,
}

pub struct CarAnalyzer {}

impl WayAnalyzer<RoadWeight> for CarAnalyzer {
    type WayProperties = CarWayProperties;

    fn analyze_way_tags(&self, tags: &Tags) -> Option<Self::WayProperties> {
        // https://wiki.openstreetmap.org/wiki/Key:highway or https://wiki.openstreetmap.org/wiki/DE:Key:highway
        // TODO: make use of `access` tag: https://wiki.openstreetmap.org/wiki/Key:access
        if let Some(highway_value) = tags.get("highway") {
            let highway_class = highway_value.to_lowercase();
            match highway_class.as_str() {
                "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary"
                | "primary_link" => Some(3.0),
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

                CarWayProperties {
                    max_speed,
                    category_weight,
                    is_bidirectional,
                }
            })
        } else {
            None
        }
    }

    fn way_edge_properties(
        &self,
        edge: H3Edge,
        way_properties: &Self::WayProperties,
    ) -> EdgeProperties<RoadWeight> {
        let weight = RoadWeight::new(
            way_properties.category_weight,
            Length::new::<meter>(cell_centroid_distance_m(edge) as f32)
                / way_properties.max_speed
                / 0.9, /* you never reach max_speed on average roads*/
        );
        EdgeProperties {
            is_bidirectional: way_properties.is_bidirectional,
            weight,
        }
    }
}

#[cfg(test)]
mod tests {
    use h3ron::H3Edge;
    use uom::si::f32::{Length, Velocity};
    use uom::si::length::meter;
    use uom::si::velocity::kilometer_per_hour;

    #[test]
    fn test_calc() {
        let speed = Velocity::new::<kilometer_per_hour>(30.0);
        let distance = Length::new::<meter>(H3Edge::edge_length_m(10) as f32);
        dbg!(distance / speed);
    }
}
