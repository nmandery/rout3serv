use h3ron_graph::graph::PreparedH3EdgeGraph;
use std::cmp::Ordering;
use std::ops::Add;

use h3ron::{H3DirectedEdge, Index};
use h3ron_graph::graph::prepared::FromIterItem;
use itertools::izip;
use num_traits::Zero;
use polars_core::frame::DataFrame;
use polars_core::prelude::{NamedFrom, UInt64Chunked};
use polars_core::series::Series;
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

const COL_EDGE: &str = "edge";
const COL_EDGE_PREFERENCE: &str = "edge_preference";
const COL_EDGE_TRAVEL_DURATION: &str = "edge_travel_duration";
const COL_LONG_EDGE: &str = "long_edge";
const COL_LONG_EDGE_PREFERENCE: &str = "long_edge_preference";
const COL_LONG_EDGE_TRAVEL_DURATION: &str = "long_edge_travel_duration";

impl ToDataFrame for PreparedH3EdgeGraph<StandardWeight> {
    fn to_dataframe(&self) -> Result<DataFrame, Error> {
        let mut directed_edges = Vec::with_capacity(self.count_edges().0);
        let mut edge_preferences = Vec::with_capacity(directed_edges.capacity());
        let mut travel_durations = Vec::with_capacity(directed_edges.capacity());
        let mut le_directed_edges = Vec::with_capacity(directed_edges.capacity());
        let mut le_edge_preferences = Vec::with_capacity(directed_edges.capacity());
        let mut le_travel_durations = Vec::with_capacity(directed_edges.capacity());

        for (edge, edgeweight) in self.iter_edges() {
            directed_edges.push(edge.h3index());
            edge_preferences.push(edgeweight.weight.edge_preference);
            travel_durations.push(edgeweight.weight.travel_duration.get::<second>());

            if let Some((longedge, longedge_weight)) = edgeweight.longedge {
                let le_edges: UInt64Chunked =
                    longedge.h3edge_path()?.map(|e| Some(e.h3index())).collect();

                le_directed_edges.push(Some(Series::new("", le_edges)));
                le_edge_preferences.push(Some(longedge_weight.edge_preference));
                le_travel_durations.push(Some(longedge_weight.travel_duration.get::<second>()));
            } else {
                le_directed_edges.push(None);
                le_edge_preferences.push(None);
                le_travel_durations.push(None);
            }
        }

        Ok(DataFrame::new(vec![
            Series::new(COL_EDGE, directed_edges),
            Series::new(COL_EDGE_PREFERENCE, edge_preferences),
            Series::new(COL_EDGE_TRAVEL_DURATION, travel_durations),
            Series::new(COL_LONG_EDGE, le_directed_edges),
            Series::new(COL_LONG_EDGE_PREFERENCE, le_edge_preferences),
            Series::new(COL_LONG_EDGE_TRAVEL_DURATION, le_travel_durations),
        ])?)
    }
}

impl FromDataFrame for PreparedH3EdgeGraph<StandardWeight> {
    fn from_dataframe(df: DataFrame) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(PreparedH3EdgeGraph::try_from_iter(
            collect_edges(df)?.into_iter(),
        )?)
    }
}

fn collect_edges(df: DataFrame) -> Result<Vec<FromIterItem<StandardWeight>>, Error> {
    let directed_edges = df.column(COL_EDGE)?.u64()?;
    let edge_preferences = df.column(COL_EDGE_PREFERENCE)?.f32()?;
    let travel_durations = df.column(COL_EDGE_TRAVEL_DURATION)?.f32()?;
    let le_directed_edges = df.column(COL_LONG_EDGE)?.list()?;
    let le_edge_preferences = df.column(COL_LONG_EDGE_PREFERENCE)?.f32()?;
    let le_travel_durations = df.column(COL_LONG_EDGE_TRAVEL_DURATION)?.f32()?;

    let mut out = Vec::with_capacity(directed_edges.len());
    for (de, de_pref, de_td, le, le_pref, le_td) in izip!(
        directed_edges.into_iter(),
        edge_preferences.into_iter(),
        travel_durations.into_iter(),
        le_directed_edges,
        le_edge_preferences.into_iter(),
        le_travel_durations.into_iter()
    ) {
        if let (Some(de), Some(de_pref), Some(de_td)) = (de, de_pref, de_td) {
            let edge = H3DirectedEdge::try_from(de)?;
            let edge_weight = StandardWeight::new(de_pref, Time::new::<second>(de_td));

            let longedge = if let (Some(le), Some(le_pref), Some(le_td)) = (le, le_pref, le_td) {
                let le = le.u64()?;
                if le.is_empty() {
                    None
                } else {
                    let le_edges = le
                        .into_iter()
                        .flatten()
                        .map(H3DirectedEdge::try_from)
                        .collect::<Result<Vec<_>, _>>()?;

                    let le_weight = StandardWeight::new(le_pref, Time::new::<second>(le_td));
                    Some((le_edges, le_weight))
                }
            } else {
                None
            };

            out.push((edge, edge_weight, longedge));
        }
    }
    Ok(out)
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
