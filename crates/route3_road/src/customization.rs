use std::cmp::Ordering;
use std::ops::{Add, Deref};
use std::sync::Arc;

use float_cmp::ApproxEqRatio;
use h3ron::{H3Cell, H3Edge, HasH3Resolution};
use h3ron_graph::graph::node::NodeType;
use h3ron_graph::graph::{EdgeWeight, GetCellNode, GetEdge, PreparedH3EdgeGraph};
use num_traits::Zero;
use rand::{thread_rng, Rng};
use uom::si::f32::Time;

use crate::weight::Weight;

#[derive(PartialOrd, PartialEq, Copy, Clone)]
pub struct RandomU32(u32);

impl Default for RandomU32 {
    fn default() -> Self {
        Self(thread_rng().gen())
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum ComparisonKind {
    /// Exact comparison
    Exact,

    /// Approximate equality comparisons bounding the ratio of the difference to the larger.
    ///
    /// Provided by [`ApproxEqRatio`].
    DifferenceRatio(f32),
}

impl Default for ComparisonKind {
    fn default() -> Self {
        Self::exact()
    }
}

impl TryFrom<f32> for ComparisonKind {
    type Error = eyre::Report;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::difference_ratio(value)
    }
}

impl ComparisonKind {
    pub fn exact() -> Self {
        Self::Exact
    }

    pub fn difference_ratio(ratio: f32) -> eyre::Result<Self> {
        if ratio < 1.0 {
            Ok(Self::DifferenceRatio(ratio))
        } else {
            Err(eyre::Report::msg(format!(
                "ratio must be < 1.0 (got {})",
                ratio
            )))
        }
    }

    pub fn compare_values(&self, this_value: f32, other_value: f32) -> Option<Ordering> {
        match self {
            ComparisonKind::Exact => this_value.partial_cmp(&other_value),
            ComparisonKind::DifferenceRatio(ratio) => {
                if this_value.approx_eq_ratio(&other_value, *ratio) {
                    Some(Ordering::Equal)
                } else {
                    this_value.partial_cmp(&other_value)
                }
            }
        }
    }

    pub fn are_equal(&self, this_value: f32, other_value: f32) -> bool {
        self.compare_values(this_value, other_value)
            .unwrap_or(Ordering::Equal)
            == Ordering::Equal
    }
}

#[derive(Copy, Clone)]
pub struct CustomizedWeight<W> {
    weight: W,
    travel_duration_cmp: ComparisonKind,
    road_category_weight_cmp: ComparisonKind,

    /// order key to maintain a consistent ordering in case
    /// the other ComparisonKind lead to non-repeatable orderings
    /// because of equality
    ord_tiebreaker: RandomU32,
}

impl<W: Weight> Add for CustomizedWeight<W>
where
    W: Zero,
{
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.weight = self.weight + rhs.weight;

        // any ComparisonKind takes precedence over the default
        // as this may be just initialized
        // to default when the instance was created using zero()
        if self.road_category_weight_cmp == ComparisonKind::default() {
            self.road_category_weight_cmp = rhs.road_category_weight_cmp
        }
        if self.travel_duration_cmp == ComparisonKind::default() {
            self.travel_duration_cmp = rhs.travel_duration_cmp
        }

        self
    }
}

impl<W: Weight> Default for CustomizedWeight<W>
where
    W: Zero,
{
    fn default() -> Self {
        Self::zero()
    }
}

impl<W: Weight> Zero for CustomizedWeight<W>
where
    W: Zero,
{
    fn zero() -> Self {
        Self {
            weight: W::zero(),
            ..Default::default()
        }
    }

    fn is_zero(&self) -> bool {
        self.weight.is_zero()
    }
}

impl<W: Weight> Deref for CustomizedWeight<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.weight
    }
}

impl<W> PartialEq<Self> for CustomizedWeight<W>
where
    W: Weight,
{
    fn eq(&self, other: &Self) -> bool {
        self.travel_duration_cmp.are_equal(
            self.weight.travel_duration().value,
            other.weight.travel_duration().value,
        ) && self.road_category_weight_cmp.are_equal(
            self.weight.category_weight(),
            other.weight.category_weight(),
        )
    }
}

impl<W> PartialOrd for CustomizedWeight<W>
where
    W: Weight,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.travel_duration_cmp
            .compare_values(
                self.weight.travel_duration().value,
                other.weight.travel_duration().value,
            )
            .map(|ordering| {
                if ordering == Ordering::Equal {
                    self.road_category_weight_cmp
                        .compare_values(
                            self.weight.category_weight(),
                            other.weight.category_weight(),
                        )
                        .map(|ordering| {
                            if ordering == Ordering::Equal {
                                self.ord_tiebreaker.partial_cmp(&other.ord_tiebreaker)
                            } else {
                                Some(ordering)
                            }
                        })
                        .flatten()
                } else {
                    Some(ordering)
                }
            })
            .flatten()
    }
}

impl<W> Weight for CustomizedWeight<W>
where
    W: Weight + Zero,
{
    fn travel_duration(&self) -> Time {
        self.weight.travel_duration()
    }

    fn category_weight(&self) -> f32 {
        self.weight.category_weight()
    }

    fn from_travel_duration(travel_duration: Time) -> Self {
        Self {
            weight: W::from_travel_duration(travel_duration),
            ..Default::default()
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
    W: Weight + Copy + Zero,
{
    type EdgeWeightType = CustomizedWeight<W>;

    fn get_edge(&self, edge: &H3Edge) -> Option<EdgeWeight<Self::EdgeWeightType>> {
        self.inner_graph
            .get_edge(edge)
            .map(|edge_weight| EdgeWeight {
                weight: CustomizedWeight {
                    weight: edge_weight.weight,
                    travel_duration_cmp: self.travel_duration_cmp,
                    road_category_weight_cmp: self.road_category_weight_cmp,
                    ..Default::default()
                },
                longedge: edge_weight.longedge.map(|(longedge, road_weight)| {
                    (
                        longedge,
                        CustomizedWeight {
                            weight: road_weight,
                            travel_duration_cmp: self.travel_duration_cmp,
                            road_category_weight_cmp: self.road_category_weight_cmp,
                            ..Default::default()
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
    use rand::prelude::SliceRandom;
    use rand::thread_rng;
    use std::cmp::Ordering;
    use uom::si::f32::Time;
    use uom::si::time::second;

    use crate::customization::{ComparisonKind, CustomizedWeight};
    use crate::RoadWeight;

    #[test]
    fn test_difference_ratio_cmp() {
        assert_eq!(
            ComparisonKind::difference_ratio(0.05)
                .unwrap()
                .compare_values(10.0, 10.0),
            Some(Ordering::Equal)
        );
        assert_eq!(
            ComparisonKind::difference_ratio(0.1)
                .unwrap()
                .compare_values(10.0, 11.0),
            Some(Ordering::Equal)
        );
        assert_eq!(
            ComparisonKind::difference_ratio(0.2)
                .unwrap()
                .compare_values(10.0, 14.0),
            Some(Ordering::Less)
        );
        assert_eq!(
            ComparisonKind::difference_ratio(0.2)
                .unwrap()
                .compare_values(10.0, 7.0),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn repeated_sort_same_result() {
        let mut cws = (0..100)
            .map(|_| {
                // separate creation of both structs instead of clone to get different `final_ord_decider`
                CustomizedWeight {
                    weight: RoadWeight::new(1.0, Time::new::<second>(10.0)),
                    travel_duration_cmp: ComparisonKind::DifferenceRatio(1.2),
                    road_category_weight_cmp: ComparisonKind::DifferenceRatio(1.2),
                    ord_tiebreaker: Default::default(),
                }
            })
            .collect::<Vec<_>>();

        cws.sort_unstable();
        let expected_tiebreaker_order =
            cws.iter().map(|cw| cw.ord_tiebreaker.0).collect::<Vec<_>>();
        //dbg!(&expected_random_key_order);

        let mut rng = thread_rng();
        for _ in 0..100 {
            cws.shuffle(&mut rng);
            cws.sort_unstable();

            let found_order = cws.iter().map(|cw| cw.ord_tiebreaker.0).collect::<Vec<_>>();
            assert_eq!(expected_tiebreaker_order, found_order);
        }
    }
}
