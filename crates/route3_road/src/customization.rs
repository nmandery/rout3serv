use std::cmp::Ordering;
use std::ops::{Add, Deref};
use std::sync::Arc;

use crate::config::{NonZeroPositiveFactor, RoutingMode};
use h3ron::{H3Cell, H3Edge, HasH3Resolution};
use h3ron_graph::graph::node::NodeType;
use h3ron_graph::graph::{EdgeWeight, GetCellNode, GetEdge, PreparedH3EdgeGraph};
use num_traits::Zero;
use uom::si::f32::Time;

use crate::weight::Weight;

// TODO: mid term: configurable road_preferences for road_types

#[derive(Copy, Clone)]
pub struct CustomizedWeight<W> {
    weight: W,
    edge_preference_factor: Option<NonZeroPositiveFactor>,
}

impl<W> CustomizedWeight<W>
where
    W: Weight,
{
    /// the calculated overall_weight to be used in comparison operations
    ///
    /// Takes all set factors into account
    fn overall_weight(&self) -> f32 {
        self.weight.travel_duration().value
            * self
                .edge_preference_factor
                .map(|epf| *epf * self.weight.edge_preference())
                .unwrap_or(1.0)
    }
}

impl<W: Weight> Add for CustomizedWeight<W>
where
    W: Zero,
{
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.weight = self.weight + rhs.weight;

        // any factors takes precedence over the default
        // as this may be just initialized
        // to default when the instance was created using zero()
        if self.edge_preference_factor.is_none() {
            self.edge_preference_factor = rhs.edge_preference_factor
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
            edge_preference_factor: None,
        }
    }

    fn is_zero(&self) -> bool {
        // factors are irrelevant for this check
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
        self.overall_weight().eq(&other.overall_weight())
    }
}

impl<W> PartialOrd for CustomizedWeight<W>
where
    W: Weight,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.overall_weight().partial_cmp(&other.overall_weight())
    }
}

impl<W> Weight for CustomizedWeight<W>
where
    W: Weight + Zero,
{
    fn travel_duration(&self) -> Time {
        self.weight.travel_duration()
    }

    fn edge_preference(&self) -> f32 {
        self.weight.edge_preference()
    }

    fn from_travel_duration(travel_duration: Time) -> Self {
        Self {
            weight: W::from_travel_duration(travel_duration),
            edge_preference_factor: None,
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
    routing_mode: RoutingMode,
}

impl<W: Sync + Send> CustomizedGraph<W> {
    pub fn set_routing_mode(&mut self, routing_mode: RoutingMode) {
        self.routing_mode = routing_mode;
    }
}

impl<W: Sync + Send> From<Arc<PreparedH3EdgeGraph<W>>> for CustomizedGraph<W> {
    fn from(inner_graph: Arc<PreparedH3EdgeGraph<W>>) -> Self {
        CustomizedGraph {
            inner_graph,
            routing_mode: RoutingMode::default(),
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
                    edge_preference_factor: self.routing_mode.edge_preference_factor,
                },
                longedge: edge_weight.longedge.map(|(longedge, road_weight)| {
                    (
                        longedge,
                        CustomizedWeight {
                            weight: road_weight,
                            edge_preference_factor: self.routing_mode.edge_preference_factor,
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
                    edge_preference_cmp: ComparisonKind::DifferenceRatio(1.2),
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
