use std::convert::TryFrom;
use std::fs::File;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ordered_float::OrderedFloat;

use route3_core::geo_types::Coordinate;
use route3_core::graph::H3Graph;
use route3_core::h3ron::H3Cell;
use route3_core::io::load_graph;
use route3_core::routing::{ManyToManyOptions, RoutingGraph};
use route3_core::WithH3Resolution;

fn load_bench_graph() -> RoutingGraph<OrderedFloat<f64>> {
    let graph: H3Graph<OrderedFloat<f64>> = load_graph(
        File::open(format!(
            "{}/../testdata/graph-germany_r7_f64.bincode",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    RoutingGraph::try_from(graph).unwrap()
}

fn route_across_germany(routing_graph: &RoutingGraph<OrderedFloat<f64>>) {
    let origin_cell = H3Cell::from_coordinate(
        &Coordinate::from((9.834909439086914, 47.68708804564653)), // Wangen im Allgäu
        routing_graph.h3_resolution(),
    )
    .unwrap();

    let destination_cells = vec![
        H3Cell::from_coordinate(
            &Coordinate::from((7.20600128173828, 53.3689915114596)), // Emden
            routing_graph.h3_resolution(),
        )
        .unwrap(),
        H3Cell::from_coordinate(
            &Coordinate::from((13.092269897460938, 54.3153216473314)), // Stralsund
            routing_graph.h3_resolution(),
        )
        .unwrap(),
    ];

    let options = ManyToManyOptions {
        num_destinations_to_reach: Some(destination_cells.len()),
        ..Default::default()
    };

    let routes_map = routing_graph
        .route_many_to_many(vec![origin_cell], destination_cells, &options)
        .unwrap();
    assert_eq!(
        routes_map.get(&origin_cell).map(|routes| routes.len()),
        Some(2)
    );
}

fn criterion_benchmark(c: &mut Criterion) {
    let routing_graph = load_bench_graph();

    let mut group = c.benchmark_group("route_many_to_many");
    // group.sample_size(10);
    group.bench_function("route_across germany", |b| {
        b.iter(|| route_across_germany(black_box(&routing_graph)))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
