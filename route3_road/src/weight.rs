use std::cmp::Ordering;
use std::ops::Add;

use gdal::vector::{Feature, FieldDefn, Layer, OGRFieldType};
use h3ron_graph::error::Error;
use h3ron_graph::io::gdal::WeightFeatureField;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use uom::si::f32::Time;
use uom::si::time::second;

pub trait Weight {
    fn travel_duration(&self) -> Time {
        Time::new::<second>(0.0)
    }

    fn category_weight(&self) -> f32 {
        0.0
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct RoadWeight {
    /// the higher the preference for the edge is, the lower is the `edge_category_weight`.
    ///
    /// Must be positive.
    road_category_weight: f32,

    /// travel duration
    travel_duration: Time,
}

impl RoadWeight {
    pub fn new(road_category_weight: f32, travel_duration: Time) -> Self {
        Self {
            road_category_weight,
            travel_duration,
        }
    }
}

impl Weight for RoadWeight {
    fn travel_duration(&self) -> Time {
        self.travel_duration
    }

    fn category_weight(&self) -> f32 {
        self.road_category_weight
    }
}

impl WeightFeatureField for RoadWeight {
    fn register_weight_fields(layer: &Layer) -> Result<(), Error> {
        let td_field_defn = FieldDefn::new("travel_duration", OGRFieldType::OFTReal)?;
        td_field_defn.add_to_layer(layer)?;
        let cw_field_defn = FieldDefn::new("category_weight", OGRFieldType::OFTReal)?;
        cw_field_defn.add_to_layer(layer)?;
        Ok(())
    }

    fn fill_weight_feature_fields<'a>(&self, feature: &mut Feature<'a>) -> Result<(), Error> {
        feature.set_field_double(
            "travel_duration",
            self.travel_duration().get::<second>() as f64,
        )?;
        feature.set_field_double("category_weight", self.road_category_weight as f64)?;
        Ok(())
    }
}

impl Add for RoadWeight {
    type Output = RoadWeight;

    fn add(mut self, rhs: Self) -> Self::Output {
        // change the category proportionally to the travel durations
        let td_self = self.travel_duration.value.abs().max(1.0);
        let td_rhs = rhs.travel_duration.value.abs().max(1.0);
        self.road_category_weight = ((self.road_category_weight.abs() * td_self)
            + (rhs.road_category_weight.abs() * td_rhs))
            / (td_self + td_rhs);

        self.travel_duration += rhs.travel_duration;
        self
    }
}

impl Zero for RoadWeight {
    fn zero() -> Self {
        Self {
            road_category_weight: 10.0,
            travel_duration: Time::new::<second>(10.0),
        }
    }

    fn is_zero(&self) -> bool {
        self.travel_duration == Time::new::<second>(0.0) && self.road_category_weight.is_zero()
    }
}

impl PartialEq for RoadWeight {
    fn eq(&self, other: &Self) -> bool {
        approx::abs_diff_eq!(
            self.travel_duration.value,
            other.travel_duration.value,
            epsilon = f32::EPSILON
        ) && approx::abs_diff_eq!(
            self.road_category_weight,
            other.road_category_weight,
            epsilon = f32::EPSILON
        )
    }
}

impl Eq for RoadWeight {}

impl PartialOrd for RoadWeight {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // TODO: decide for the better road_category when the difference in travel_duration is
        //       less than N seconds?
        match self.travel_duration.partial_cmp(&other.travel_duration) {
            Some(Ordering::Equal) => self
                .road_category_weight
                .partial_cmp(&other.road_category_weight),
            Some(v) => Some(v),
            None => None,
        }
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
