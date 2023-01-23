use geo::{Coord, LineString, MultiLineString};
use h3o::{CellIndex, DirectedEdgeIndex, LatLng};

/// converts `&[DirectedEdgeIndex]` slices to [`MultiLineString`] while attempting
/// to combine consequent [`DirectedEdgeIndex`] values into a single [`LineString`]
pub fn edges_to_multilinestring(edges: impl Iterator<Item = DirectedEdgeIndex>) -> MultiLineString {
    celltuples_to_multlinestring(edges.map(|edge| (edge.origin(), edge.destination())))
}

/// convert an iterator of subsequent [`CellIndex`]-tuples `(origin_cell, destination_cell)` generated
/// from [`DirectedEdgeIndex`] values to a multilinestring
fn celltuples_to_multlinestring<I>(iter: I) -> MultiLineString
where
    I: IntoIterator<Item = (CellIndex, CellIndex)>,
{
    let mut linestrings = vec![];
    let mut last_destination_cell: Option<CellIndex> = None;
    let mut coordinates: Vec<Coord> = Vec::with_capacity(20);
    for (origin_cell, destination_cell) in iter {
        if coordinates.is_empty() {
            coordinates.push(LatLng::from(origin_cell).into());
        } else if last_destination_cell != Some(origin_cell) {
            // create a new linestring
            linestrings.push(LineString::from(std::mem::take(&mut coordinates)));
            coordinates.push(LatLng::from(origin_cell).into());
        }
        coordinates.push(LatLng::from(destination_cell).into());
        last_destination_cell = Some(destination_cell);
    }
    if !coordinates.is_empty() {
        linestrings.push(LineString::from(coordinates));
    }
    MultiLineString::new(linestrings)
}
