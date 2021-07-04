use std::borrow::Borrow;
use std::cmp::Ordering;

use crate::error::Error;
use crate::h3ron::{H3Cell, Index};

// how th write a function which accepts an iterator: https://stackoverflow.com/questions/57543399/how-to-a-pass-iterators-to-a-function-in-rust

/// change the resolution of the given h3 cells to the `target_resolution`
pub fn change_h3_resolution<I>(
    cell_iter: I,
    target_h3_resolution: u8,
) -> ChangeH3ResolutionIterator<<I as IntoIterator>::IntoIter>
where
    I: IntoIterator,
    I::Item: Borrow<H3Cell>,
{
    ChangeH3ResolutionIterator {
        inner: cell_iter.into_iter(),
        target_h3_resolution,
        current_batch: Default::default(),
    }
}

pub struct ChangeH3ResolutionIterator<I> {
    inner: I,
    target_h3_resolution: u8,
    current_batch: Vec<H3Cell>,
}

impl<I> Iterator for ChangeH3ResolutionIterator<I>
where
    I: Iterator,
    I::Item: Borrow<H3Cell>,
{
    type Item = H3Cell;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cell) = self.current_batch.pop() {
            Some(cell)
        } else if let Some(cell) = self.inner.next() {
            match cell.borrow().resolution().cmp(&self.target_h3_resolution) {
                Ordering::Less => {
                    self.current_batch = cell.borrow().get_children(self.target_h3_resolution);
                    self.current_batch.pop()
                }
                Ordering::Equal => Some(*cell.borrow()),
                Ordering::Greater => Some(
                    cell.borrow()
                        .get_parent_unchecked(self.target_h3_resolution),
                ),
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::algo::iter::change_h3_resolution;
    use crate::h3ron::Index;
    use geo::Coordinate;
    use h3ron::H3Cell;
    use std::iter::once;

    #[test]
    fn test_change_h3_resolution_same_res() {
        let cell = H3Cell::from_coordinate(&Coordinate::from((12.3, 45.4)), 6).unwrap();
        let changed = change_h3_resolution(once(cell), 6).collect::<Vec<_>>();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0], cell);
    }

    #[test]
    fn test_change_h3_resolution_lower_res() {
        let cell = H3Cell::from_coordinate(&Coordinate::from((12.3, 45.4)), 6).unwrap();
        let changed = change_h3_resolution(once(cell), 5).collect::<Vec<_>>();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].resolution(), 5);
    }

    #[test]
    fn test_change_h3_resolution_higher_res() {
        let cell = H3Cell::from_coordinate(&Coordinate::from((12.3, 45.4)), 6).unwrap();
        let changed = change_h3_resolution(once(cell), 7).collect::<Vec<_>>();
        assert_eq!(changed.len(), 7);
        assert_eq!(changed[0].resolution(), 7);
    }
}
