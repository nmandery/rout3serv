use std::cmp::Ordering;
use std::ops::Add;

use gdal::vector::{Feature, FieldDefn, Layer, OGRFieldType};
use h3ron_graph::error::Error;
use h3ron_graph::io::gdal::WeightFeatureField;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use uom::si::f32::Time;
use uom::si::time::second;

use crate::grpc::ServerWeight;

pub trait Weight {
    fn travel_duration(&self) -> Time {
        Time::new::<second>(0.0)
    }

    fn edge_preference(&self) -> f32 {
        0.0
    }

    fn from_travel_duration(travel_duration: Time) -> Self;
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct RoadWeight {
    /// the higher the preference for the edge is, the lower is the `edge_preference`.
    ///
    /// Must be positive.
    #[serde(rename = "rcw")]
    edge_preference: f32,

    /// travel duration
    #[serde(rename = "td")]
    travel_duration: Time,
}

impl RoadWeight {
    pub fn new(edge_preference: f32, travel_duration: Time) -> Self {
        Self {
            edge_preference,
            travel_duration,
        }
    }
}

impl Weight for RoadWeight {
    fn travel_duration(&self) -> Time {
        self.travel_duration
    }

    fn edge_preference(&self) -> f32 {
        self.edge_preference
    }

    fn from_travel_duration(travel_duration: Time) -> Self {
        Self {
            edge_preference: 0.0,
            travel_duration,
        }
    }
}

impl ServerWeight for RoadWeight {}

impl WeightFeatureField for RoadWeight {
    fn register_weight_fields(layer: &Layer) -> Result<(), Error> {
        let td_field_defn = FieldDefn::new("travel_duration", OGRFieldType::OFTReal)?;
        td_field_defn.add_to_layer(layer)?;
        let cw_field_defn = FieldDefn::new("edge_preference", OGRFieldType::OFTReal)?;
        cw_field_defn.add_to_layer(layer)?;
        Ok(())
    }

    fn fill_weight_feature_fields<'a>(&self, feature: &mut Feature<'a>) -> Result<(), Error> {
        feature.set_field_double(
            "travel_duration",
            self.travel_duration().get::<second>() as f64,
        )?;
        feature.set_field_double("edge_preference", self.edge_preference as f64)?;
        Ok(())
    }
}

impl Add for RoadWeight {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        // change the category proportionally to the travel durations
        let td_self = self.travel_duration.value.abs().max(1.0);
        let td_rhs = rhs.travel_duration.value.abs().max(1.0);
        self.edge_preference = self
            .edge_preference
            .abs()
            .mul_add(td_self, rhs.edge_preference.abs() * td_rhs)
            / (td_self + td_rhs);

        self.travel_duration += rhs.travel_duration;
        self
    }
}

impl Zero for RoadWeight {
    fn zero() -> Self {
        Self {
            edge_preference: 10.0,
            travel_duration: Time::new::<second>(1.0),
        }
    }

    fn is_zero(&self) -> bool {
        self.travel_duration == Time::new::<second>(0.0) && self.edge_preference.is_zero()
    }
}

impl PartialEq for RoadWeight {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).unwrap_or(Ordering::Equal) == Ordering::Equal
    }
}

impl Eq for RoadWeight {}

impl PartialOrd for RoadWeight {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.travel_duration
            .value
            .partial_cmp(&other.travel_duration.value)
            .and_then(|ordering| {
                if ordering == Ordering::Equal {
                    self.edge_preference.partial_cmp(&other.edge_preference)
                } else {
                    Some(ordering)
                }
            })
    }
}

impl Ord for RoadWeight {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use uom::si::f32::Time;
    use uom::si::time::second;

    use crate::weight::RoadWeight;

    macro_rules! secs {
        ($s:expr) => {
            Time::new::<second>($s as f32)
        };
    }
    #[test]
    fn roadweight_partial_eq() {
        assert!(RoadWeight::new(3.0, secs!(30)) > RoadWeight::new(2.0, secs!(29)));
        assert!(RoadWeight::new(3.0, secs!(30)) > RoadWeight::new(3.0, secs!(20)));
        assert!(RoadWeight::new(2.0, secs!(30)) > RoadWeight::new(3.0, secs!(20)));
    }

    #[test]
    fn roadweight_add() {
        let rw1 = RoadWeight::new(4.0, secs!(10));
        let rw2 = RoadWeight::new(6.0, secs!(15));
        assert_eq!(rw1 + rw2, RoadWeight::new(5.2, secs!(25)));
    }
}
