use std::cmp::Ordering;
use std::ops::{Add, Deref};
use std::sync::Arc;

use h3ron::{H3Cell, H3Edge, HasH3Resolution};
use h3ron_graph::graph::node::NodeType;
use h3ron_graph::graph::{EdgeWeight, GetCellNode, GetEdge, PreparedH3EdgeGraph};
use num_traits::Zero;
use uom::si::f32::Time;

use crate::weight::Weight;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum ComparisonKind {
    Exact,
    Fuzzy(f32),
}

impl Default for ComparisonKind {
    fn default() -> Self {
        Self::Exact
    }
}

impl TryFrom<f32> for ComparisonKind {
    type Error = eyre::Report;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if value < 1.0 {
            Err(Self::Error::msg(format!(
                "fuzzy percentage must be >= 1.0 (got {})",
                value
            )))
        } else {
            Ok(Self::Fuzzy(value))
        }
    }
}

impl ComparisonKind {
    pub fn exact() -> Self {
        Self::default()
    }

    pub fn fuzzy(fuzzy_percentage: f32) -> eyre::Result<Self> {
        fuzzy_percentage.try_into()
    }

    pub fn abs_partial_cmp(&self, mut this_value: f32, mut other_value: f32) -> Option<Ordering> {
        match self {
            ComparisonKind::Exact => this_value.partial_cmp(&other_value),
            ComparisonKind::Fuzzy(fuzzy_percentage) => {
                this_value = this_value.abs();
                other_value = other_value.abs();

                Some(if this_value < other_value {
                    if (other_value / this_value) > *fuzzy_percentage {
                        Ordering::Less
                    } else {
                        Ordering::Equal
                    }
                } else if (this_value / other_value) > *fuzzy_percentage {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                })
            }
        }
    }

    pub fn abs_partial_eq(&self, this_value: f32, other_value: f32) -> bool {
        self.abs_partial_cmp(this_value, other_value)
            .unwrap_or(Ordering::Equal)
            == Ordering::Equal
    }
}

#[derive(Copy, Clone)]
pub struct CustomizedWeight<W> {
    road_weight: W,
    travel_duration_cmp: ComparisonKind,
    road_category_weight_cmp: ComparisonKind,
}

impl<W: Weight> Add for CustomizedWeight<W>
where
    W: Zero,
{
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.road_weight = self.road_weight + rhs.road_weight;

        // any ComparisonKind takes precedence over Exact
        // as Exact is default and so this may be just initialized
        // to default when the instance was created using zero()
        if self.road_category_weight_cmp == ComparisonKind::Exact {
            self.road_category_weight_cmp = rhs.road_category_weight_cmp
        }
        if self.travel_duration_cmp == ComparisonKind::Exact {
            self.travel_duration_cmp = rhs.travel_duration_cmp
        }
        self
    }
}

impl<W: Weight> Zero for CustomizedWeight<W>
where
    W: Zero,
{
    fn zero() -> Self {
        Self {
            road_weight: W::zero(),
            travel_duration_cmp: Default::default(),
            road_category_weight_cmp: Default::default(),
        }
    }

    fn is_zero(&self) -> bool {
        self.road_weight.is_zero()
    }
}

impl<W: Weight> Deref for CustomizedWeight<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.road_weight
    }
}

impl<W> PartialEq<Self> for CustomizedWeight<W>
where
    W: Weight,
{
    fn eq(&self, other: &Self) -> bool {
        self.travel_duration_cmp.abs_partial_eq(
            self.road_weight.travel_duration().value,
            other.road_weight.travel_duration().value,
        ) && self.road_category_weight_cmp.abs_partial_eq(
            self.road_weight.category_weight(),
            other.road_weight.category_weight(),
        )
    }
}

impl<W> PartialOrd for CustomizedWeight<W>
where
    W: Weight,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.travel_duration_cmp
            .abs_partial_cmp(
                self.road_weight.travel_duration().value,
                other.road_weight.travel_duration().value,
            )
            .map(|ordering| {
                if ordering == Ordering::Equal {
                    self.road_category_weight_cmp.abs_partial_cmp(
                        self.road_weight.category_weight(),
                        other.road_weight.category_weight(),
                    )
                } else {
                    Some(ordering)
                }
            })
            .flatten()
    }
}

impl<W> Weight for CustomizedWeight<W>
where
    W: Weight,
{
    fn travel_duration(&self) -> Time {
        self.road_weight.travel_duration()
    }

    fn category_weight(&self) -> f32 {
        self.road_weight.category_weight()
    }

    fn from_travel_duration(travel_duration: Time) -> Self {
        Self {
            road_weight: W::from_travel_duration(travel_duration),
            travel_duration_cmp: Default::default(),
            road_category_weight_cmp: Default::default(),
        }
    }
}

impl<W: Weight> Eq for CustomizedWeight<W> {}

impl<W: Weight> Ord for CustomizedWeight<W> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// A prepared graph with customized weight comparisons
pub struct CustomizedGraph<W: Sync + Send> {
    inner_graph: Arc<PreparedH3EdgeGraph<W>>,
    pub travel_duration_cmp: ComparisonKind,
    pub road_category_weight_cmp: ComparisonKind,
}

impl<W: Sync + Send> From<Arc<PreparedH3EdgeGraph<W>>> for CustomizedGraph<W> {
    fn from(inner_graph: Arc<PreparedH3EdgeGraph<W>>) -> Self {
        CustomizedGraph {
            inner_graph,
            travel_duration_cmp: Default::default(),
            road_category_weight_cmp: Default::default(),
        }
    }
}

impl<W: Sync + Send> GetCellNode for CustomizedGraph<W> {
    fn get_cell_node(&self, cell: &H3Cell) -> Option<NodeType> {
        self.inner_graph.get_cell_node(cell)
    }
}

impl<W: Sync + Send> GetEdge for CustomizedGraph<W>
where
    W: Weight + Copy,
{
    type EdgeWeightType = CustomizedWeight<W>;

    fn get_edge(&self, edge: &H3Edge) -> Option<EdgeWeight<Self::EdgeWeightType>> {
        self.inner_graph
            .get_edge(edge)
            .map(|edge_weight| EdgeWeight {
                weight: CustomizedWeight {
                    road_weight: edge_weight.weight,
                    travel_duration_cmp: self.travel_duration_cmp,
                    road_category_weight_cmp: self.road_category_weight_cmp,
                },
                longedge: edge_weight.longedge.map(|(longedge, road_weight)| {
                    (
                        longedge,
                        CustomizedWeight {
                            road_weight,
                            travel_duration_cmp: self.travel_duration_cmp,
                            road_category_weight_cmp: self.road_category_weight_cmp,
                        },
                    )
                }),
            })
    }
}

impl<W: Sync + Send> HasH3Resolution for CustomizedGraph<W> {
    fn h3_resolution(&self) -> u8 {
        self.inner_graph.h3_resolution()
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use crate::customization::ComparisonKind;

    #[test]
    fn test_fuzzy_cmp() {
        assert_eq!(
            ComparisonKind::fuzzy(1.0)
                .unwrap()
                .abs_partial_cmp(10.0, 10.0),
            Some(Ordering::Equal)
        );
        assert_eq!(
            ComparisonKind::fuzzy(1.2)
                .unwrap()
                .abs_partial_cmp(10.0, 11.0),
            Some(Ordering::Equal)
        );
        assert_eq!(
            ComparisonKind::fuzzy(1.2)
                .unwrap()
                .abs_partial_cmp(10.0, 14.0),
            Some(Ordering::Less)
        );
        assert_eq!(
            ComparisonKind::fuzzy(1.2)
                .unwrap()
                .abs_partial_cmp(10.0, 7.0),
            Some(Ordering::Greater)
        );
    }
}
