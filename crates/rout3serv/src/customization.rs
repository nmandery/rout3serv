use h3o::{CellIndex, DirectedEdgeIndex, Resolution};
use std::cmp::Ordering;
use std::ops::{Add, Deref};
use std::sync::Arc;

use hexigraph::graph::node::NodeType;
use hexigraph::graph::{EdgeWeight, GetCellEdges, GetCellNode, PreparedH3EdgeGraph};
use hexigraph::HasH3Resolution;
use num_traits::Zero;
use uom::si::f32::Time;

use crate::config::{NonZeroPositiveFactor, RoutingMode};
use crate::weight::{StandardWeight, Weight};

// TODO: mid term: configurable road_preferences for road_types

#[derive(Copy, Clone)]
pub struct CustomizedWeight {
    weight: StandardWeight,
    edge_preference_factor: Option<NonZeroPositiveFactor>,
}

impl CustomizedWeight {
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

impl Add for CustomizedWeight {
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

impl Default for CustomizedWeight {
    fn default() -> Self {
        Self::zero()
    }
}

impl Zero for CustomizedWeight {
    fn zero() -> Self {
        Self {
            weight: StandardWeight::zero(),
            edge_preference_factor: None,
        }
    }

    fn is_zero(&self) -> bool {
        // factors are irrelevant for this check
        self.weight.is_zero()
    }
}

impl Deref for CustomizedWeight {
    type Target = StandardWeight;

    fn deref(&self) -> &Self::Target {
        &self.weight
    }
}

impl PartialEq<Self> for CustomizedWeight {
    fn eq(&self, other: &Self) -> bool {
        self.overall_weight().eq(&other.overall_weight())
    }
}

impl PartialOrd for CustomizedWeight {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.overall_weight().partial_cmp(&other.overall_weight())
    }
}

impl Weight for CustomizedWeight {
    fn travel_duration(&self) -> Time {
        self.weight.travel_duration()
    }

    fn edge_preference(&self) -> f32 {
        self.weight.edge_preference()
    }

    fn from_travel_duration(travel_duration: Time) -> Self {
        Self {
            weight: StandardWeight::from_travel_duration(travel_duration),
            edge_preference_factor: None,
        }
    }
}

impl Eq for CustomizedWeight {}

impl Ord for CustomizedWeight {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// A prepared graph with customized weight comparisons
pub struct CustomizedGraph {
    inner_graph: Arc<PreparedH3EdgeGraph<StandardWeight>>,
    routing_mode: RoutingMode,
}

impl CustomizedGraph {
    pub fn set_routing_mode(&mut self, routing_mode: RoutingMode) {
        self.routing_mode = routing_mode;
    }
}

impl From<Arc<PreparedH3EdgeGraph<StandardWeight>>> for CustomizedGraph {
    fn from(inner_graph: Arc<PreparedH3EdgeGraph<StandardWeight>>) -> Self {
        CustomizedGraph {
            inner_graph,
            routing_mode: RoutingMode::default(),
        }
    }
}

impl GetCellNode for CustomizedGraph {
    fn get_cell_node(&self, cell: CellIndex) -> Option<NodeType> {
        self.inner_graph.get_cell_node(cell)
    }
}

impl GetCellEdges for CustomizedGraph {
    type EdgeWeightType = CustomizedWeight;

    fn get_edges_originating_from(
        &self,
        cell: CellIndex,
    ) -> Vec<(DirectedEdgeIndex, EdgeWeight<Self::EdgeWeightType>)> {
        self.inner_graph
            .get_edges_originating_from(cell)
            .into_iter()
            .map(|(edge, edge_weight)| {
                (
                    edge,
                    EdgeWeight {
                        weight: CustomizedWeight {
                            weight: edge_weight.weight,
                            edge_preference_factor: self.routing_mode.edge_preference_factor,
                        },
                        fastforward: edge_weight.fastforward.map(|(fastforward, road_weight)| {
                            (
                                fastforward,
                                CustomizedWeight {
                                    weight: road_weight,
                                    edge_preference_factor: self
                                        .routing_mode
                                        .edge_preference_factor,
                                },
                            )
                        }),
                    },
                )
            })
            .collect()
    }
}

impl HasH3Resolution for CustomizedGraph {
    fn h3_resolution(&self) -> Resolution {
        self.inner_graph.h3_resolution()
    }
}
