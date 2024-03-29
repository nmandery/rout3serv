//! WayAnalyzer for pedestrians.
//!
//! This is just a very simple implementation - to be improved in the future.
//!
//! Ideas for improvements:
//! - use a DEM (Copernicus DEM-30) to derive inclines and declines in height
//!   along edges. This could also be of use for other models.
//!
use h3o::DirectedEdgeIndex;
use hexigraph::algorithm::edge::cell_centroid_distance_m;
use hexigraph::io::osm::osmpbfreader::Tags;
use hexigraph::io::osm::{EdgeProperties, WayAnalyzer};
use uom::si::f32::Length;
use uom::si::length::meter;

use crate::osm::tags::sidewalk::infer_sidewalk;
use crate::osm::WALKING_SPEED;
use crate::StandardWeight;

pub struct FootwayProperties {
    edge_preference: f32,
}

pub struct FootwayAnalyzer {}

impl WayAnalyzer<StandardWeight> for FootwayAnalyzer {
    type WayProperties = FootwayProperties;

    fn analyze_way_tags(
        &self,
        tags: &Tags,
    ) -> Result<Option<Self::WayProperties>, hexigraph::error::Error> {
        // https://wiki.openstreetmap.org/wiki/Key:highway or https://wiki.openstreetmap.org/wiki/DE:Key:highway
        // TODO: make use of `access` tag: https://wiki.openstreetmap.org/wiki/Key:access
        let mut edge_preference = None;

        if let Some(highway_value) = tags.get("highway") {
            edge_preference = match highway_value.to_lowercase().as_str() {
                "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary"
                | "primary_link" => infer_sidewalk(tags).map(|_| 10.0),
                "secondary" | "secondary_link" | "tertiary" | "tertiary_link" => {
                    infer_sidewalk(tags).map(|_| 6.0)
                }
                "road" => infer_sidewalk(tags).map(|_| 4.0),

                "unclassified" | "residential" | "living_street" | "service" | "rural" => {
                    match infer_sidewalk(tags) {
                        None => Some(2.0),
                        Some(_) => Some(1.0),
                    }
                }
                "pedestrian" | "footway" | "track" | "path" | "steps" => Some(1.0),
                _ => None,
            };
        }

        if let Some(footway_value) = tags.get("footway") {
            edge_preference = match footway_value.to_lowercase().as_str() {
                "sidewalk" | "crossing" => Some(1.0),
                _ => edge_preference,
            };
        }

        Ok(edge_preference.map(|rcw| FootwayProperties {
            edge_preference: rcw,
        }))
    }

    fn way_edge_properties(
        &self,
        edge: DirectedEdgeIndex,
        way_properties: &Self::WayProperties,
    ) -> Result<EdgeProperties<StandardWeight>, hexigraph::error::Error> {
        let weight = StandardWeight::new(
            way_properties.edge_preference,
            Length::new::<meter>(cell_centroid_distance_m(edge) as f32) / *WALKING_SPEED,
        );
        Ok(EdgeProperties {
            is_bidirectional: true,
            weight,
        })
    }
}
