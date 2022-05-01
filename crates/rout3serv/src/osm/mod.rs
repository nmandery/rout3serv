use once_cell::sync::Lazy;
use uom::si::f32::Velocity;
use uom::si::velocity::kilometer_per_hour;

pub mod car;
pub mod pedestrian;
pub mod tags;

/// > Many people tend to walk at about 1.42 metres per second (5.1 km/h; 3.2 mph; 4.7 ft/s).
/// >
/// > ...
/// >
/// > ## In urban design
/// > The typical walking speed of 1.4 metres per second (5.0 km/h; 3.1 mph; 4.6 ft/s) is
/// > recommended by design guides including the Design Manual for Roads and Bridges.
/// > Transport for London recommend 1.33 metres per second (4.8 km/h; 3.0 mph; 4.4 ft/s)
/// > in the PTAL methodology.
///
/// From [https://en.wikipedia.org/wiki/Preferred_walking_speed]
pub static WALKING_SPEED: Lazy<Velocity> = Lazy::new(|| Velocity::new::<kilometer_per_hour>(5.0));
