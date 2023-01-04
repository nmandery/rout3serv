use h3ron_graph::graph::PreparedH3EdgeGraph;
use std::cmp::Ordering;
use std::ops::Add;

use num_traits::Zero;
use polars_core::frame::DataFrame;
use serde::{Deserialize, Serialize};
use uom::si::f32::Time;
use uom::si::time::second;

use crate::grpc::ServerWeight;
use crate::io::dataframe::{FromDataFrame, ToDataFrame};
use crate::io::Error;

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
pub struct StandardWeight {
    /// the higher the preference for the edge is, the lower is the `edge_preference`.
    ///
    /// Must be positive.
    #[serde(rename = "rcw")]
    edge_preference: f32,

    /// travel duration
    #[serde(rename = "td")]
    travel_duration: Time,
}

impl StandardWeight {
    pub fn new(edge_preference: f32, travel_duration: Time) -> Self {
        Self {
            edge_preference,
            travel_duration,
        }
    }
}

impl Weight for StandardWeight {
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

impl ServerWeight for StandardWeight {}

impl Add for StandardWeight {
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

impl Zero for StandardWeight {
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

impl PartialEq for StandardWeight {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).unwrap_or(Ordering::Equal) == Ordering::Equal
    }
}

impl Eq for StandardWeight {}

impl PartialOrd for StandardWeight {
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

impl Ord for StandardWeight {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl ToDataFrame for PreparedH3EdgeGraph<StandardWeight> {
    fn to_dataframe(&self) -> Result<DataFrame, Error> {
        todo!()
    }
}

impl FromDataFrame for PreparedH3EdgeGraph<StandardWeight> {
    fn from_dataframe(df: DataFrame) -> Result<Self, Error>
    where
        Self: Sized,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use uom::si::f32::Time;
    use uom::si::time::second;

    use crate::weight::StandardWeight;

    macro_rules! secs {
        ($s:expr) => {
            Time::new::<second>($s as f32)
        };
    }
    #[test]
    fn roadweight_partial_eq() {
        assert!(StandardWeight::new(3.0, secs!(30)) > StandardWeight::new(2.0, secs!(29)));
        assert!(StandardWeight::new(3.0, secs!(30)) > StandardWeight::new(3.0, secs!(20)));
        assert!(StandardWeight::new(2.0, secs!(30)) > StandardWeight::new(3.0, secs!(20)));
    }

    #[test]
    fn roadweight_add() {
        let rw1 = StandardWeight::new(4.0, secs!(10));
        let rw2 = StandardWeight::new(6.0, secs!(15));
        assert_eq!(rw1 + rw2, StandardWeight::new(5.2, secs!(25)));
    }
}
