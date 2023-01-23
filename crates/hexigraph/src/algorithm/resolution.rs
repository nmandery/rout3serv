use std::borrow::Borrow;
use std::cmp::Ordering;

use h3o::{CellIndex, Resolution};

/// Returns an iterator to change the resolution of the given cells to the `output_h3_resolution`.
pub fn transform_resolution<I>(
    input_iter: I,
    output_h3_resolution: Resolution,
) -> impl Iterator<Item = CellIndex>
where
    I: IntoIterator,
    I::Item: Borrow<CellIndex>,
{
    let out_r: u8 = output_h3_resolution.into();
    input_iter
        .into_iter()
        .flat_map(move |item| -> Box<dyn Iterator<Item = CellIndex>> {
            let cell = item.borrow();
            let cell_r: u8 = cell.resolution().into();
            match cell_r.cmp(&out_r) {
                Ordering::Equal => Box::new(Some(*cell).into_iter()),
                Ordering::Less => Box::new(cell.children(output_h3_resolution)),
                Ordering::Greater => Box::new(cell.parent(output_h3_resolution).into_iter()),
            }
        })
}

#[cfg(test)]
mod tests {
    use std::iter::once;

    use geo::Coord;
    use h3o::LatLng;

    use super::*;

    #[test]
    fn transform_resolution_same_res() {
        let cell = LatLng::try_from(Coord::from((12.3, 45.4)))
            .unwrap()
            .to_cell(Resolution::Six);
        let changed: Vec<_> = transform_resolution(once(cell), Resolution::Six).collect();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0], cell);
    }

    #[test]
    fn transform_resolution_lower_res() {
        let cell = LatLng::try_from(Coord::from((12.3, 45.4)))
            .unwrap()
            .to_cell(Resolution::Six);
        let changed: Vec<_> = transform_resolution(once(cell), Resolution::Five).collect();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].resolution(), Resolution::Five);
    }

    #[test]
    fn transform_resolution_higher_res() {
        let cell = LatLng::try_from(Coord::from((12.3, 45.4)))
            .unwrap()
            .to_cell(Resolution::Six);
        let changed: Vec<_> = transform_resolution(once(cell), Resolution::Seven).collect();
        assert_eq!(changed.len(), 7);
        assert_eq!(changed[0].resolution(), Resolution::Seven);
    }
}
