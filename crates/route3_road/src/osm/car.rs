use h3ron::H3Edge;
use h3ron_graph::formats::osm::osmpbfreader::Tags;
use h3ron_graph::formats::osm::{EdgeProperties, WayAnalyzer};
use uom::si::f32::{Length, Velocity};
use uom::si::length::meter;
use uom::si::velocity::kilometer_per_hour;

use crate::osm::tags::maxspeed::{infer_maxspeed, MaxSpeed};
use crate::weight::RoadWeight;

pub struct CarWayProperties {
    max_speed: Velocity,
    edge_preference: f32,
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
            let (category_weight, estimated_speed_reduction_percent) = match highway_class.as_str()
            {
                "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary"
                | "primary_link" => (3.0, 1.0),
                "secondary" | "secondary_link" => (4.0, 0.9),
                "tertiary" | "tertiary_link" => (5.0, 0.8),
                "unclassified" | "residential" | "living_street" | "service" | "rural" => {
                    (8.0, 0.95)
                }
                "road" => (9.0, 0.9),
                // "track" => Some(200.0), // mostly non-public agriculture/forestry roads
                "pedestrian" | "footway" => (50.0, 1.0), // fussgängerzone
                _ => return None,
            };
            // oneway streets (https://wiki.openstreetmap.org/wiki/Key:oneway)
            // NOTE: reversed direction "oneway=-1" is not supported
            let is_bidirectional = tags
                .get("oneway")
                .map(|v| v.to_lowercase() != "yes")
                .unwrap_or(true);

            let max_speed = match infer_maxspeed(tags, &highway_class) {
                MaxSpeed::Limited(v) => v,
                MaxSpeed::Unlimited => Velocity::new::<kilometer_per_hour>(130.0),
                MaxSpeed::Unknown => Velocity::new::<kilometer_per_hour>(40.0),
            } * estimated_speed_reduction_percent;

            Some(CarWayProperties {
                max_speed,
                edge_preference: category_weight,
                is_bidirectional,
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
            way_properties.edge_preference,
            Length::new::<meter>(edge.cell_centroid_distance_m() as f32) / way_properties.max_speed,
        );
        EdgeProperties {
            is_bidirectional: way_properties.is_bidirectional,
            weight,
        }
    }
}

#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use h3ron::H3Edge;
    use uom::si::f32::{Length, Velocity};
    use uom::si::length::meter;
    use uom::si::velocity::kilometer_per_hour;

    #[test]
    fn test_calc() {
        let speed = Velocity::new::<kilometer_per_hour>(30.0);
        let distance = Length::new::<meter>(H3Edge::edge_length_m(6) as f32);

        let travel_time = distance / speed;
        assert!(approx_eq!(f32, travel_time.value, 387.5379f32));
        dbg!(travel_time);
    }
}
