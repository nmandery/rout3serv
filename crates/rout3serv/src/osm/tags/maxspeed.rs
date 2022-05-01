use once_cell::sync::Lazy;
use std::convert::Infallible;
use std::str::FromStr;

use crate::osm::WALKING_SPEED;
use h3ron_graph::formats::osm::osmpbfreader::Tags;
use regex::{Captures, Regex};
use uom::si::f32::Velocity;
use uom::si::velocity::{kilometer_per_hour, knot, meter_per_second, mile_per_hour};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MaxSpeed {
    Limited(Velocity),
    Unlimited,
    Unknown,
}

impl MaxSpeed {
    pub fn new_limited_kmh(value: f32) -> Self {
        MaxSpeed::Limited(Velocity::new::<kilometer_per_hour>(value))
    }

    pub fn new_limited(value: Velocity) -> Self {
        MaxSpeed::Limited(value)
    }

    pub fn known_or_else<F: FnOnce() -> Self>(self, f: F) -> Self {
        match self {
            Self::Unknown => f(),
            Self::Limited(_) | Self::Unlimited => self,
        }
    }

    #[allow(dead_code)]
    pub fn velocity(&self) -> Option<Velocity> {
        match self {
            Self::Limited(v) => Some(*v),
            _ => None,
        }
    }
}

impl Default for MaxSpeed {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<Velocity> for MaxSpeed {
    fn from(value: Velocity) -> Self {
        MaxSpeed::Limited(value)
    }
}

pub fn infer_maxspeed(tags: &Tags, highway_class: &str) -> MaxSpeed {
    tags.get("maxspeed") // most specific limit first
        .map(|value| MaxSpeed::from_str(value.as_str()).unwrap())
        .unwrap_or_default()
        .known_or_else(|| {
            tags.get("zone:maxspeed") // general limit for the zone
                .map(|value| MaxSpeed::from_str(value.as_str()).unwrap())
                .unwrap_or_default()
        })
        .known_or_else(|| {
            match highway_class {
                // TODO: use this to derive the category
                // TODO: most of these values are germany specific
                "motorway" | "motorway_link" | "primary" | "primary_link" => {
                    MaxSpeed::new_limited_kmh(120.0)
                }
                "rural" | "tertiary" | "tertiary_link" => MaxSpeed::new_limited_kmh(80.0),
                "trunk" | "trunk_link" | "secondary" | "secondary_link" => {
                    MaxSpeed::new_limited_kmh(100.0)
                }
                "urban" | "road" | "unclassified" => MaxSpeed::new_limited_kmh(50.0),
                "pedestrian" | "footway" | "path" => MaxSpeed::new_limited(*WALKING_SPEED),
                "living_street" => MaxSpeed::new_limited_kmh(7.0),
                "bicycle_road" | "service" | "residential" | "track" => {
                    MaxSpeed::new_limited_kmh(30.0)
                }
                //"track" => 20.0.into(), // mostly non-public agriculture/forestry roads
                _ => MaxSpeed::Unknown,
            }
        })
}

static RE_MAXSPEED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([A-Za-z\-]+:(zone:?)?)?(?P<value>[1-9][0-9]*)(\s*(?P<units>[a-zA-Z/]+))?")
        .unwrap()
});

impl FromStr for MaxSpeed {
    type Err = Infallible;

    /// parse the OSM "maxspeed" tag contents
    ///
    /// based on parts of the "Implicit max speed values", used to
    /// parse the contents of the "zone:maxspeed" tag.
    /// from <https://wiki.openstreetmap.org/wiki/Key:maxspeed>
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ms = match s.to_lowercase().trim() {
            "walk" => MaxSpeed::new_limited_kmh(5.0),
            "none" => MaxSpeed::Unlimited,
            "" => MaxSpeed::Unknown,
            "at:bicycle_road" => MaxSpeed::new_limited_kmh(30.0),
            "at:motorway" => MaxSpeed::new_limited_kmh(130.0),
            "at:rural" => MaxSpeed::new_limited_kmh(100.0),
            "at:trunk" => MaxSpeed::new_limited_kmh(100.0),
            "at:urban" => MaxSpeed::new_limited_kmh(50.0),
            "be-bru:rural" => MaxSpeed::new_limited_kmh(70.0),
            "be-bru:urban" => MaxSpeed::new_limited_kmh(30.0),
            "be:cyclestreet" => MaxSpeed::new_limited_kmh(30.0),
            "be:living_street" => MaxSpeed::new_limited_kmh(20.0),
            "be:motorway" => MaxSpeed::new_limited_kmh(120.0),
            "be:trunk" => MaxSpeed::new_limited_kmh(120.0),
            "be-vlg:rural" => MaxSpeed::new_limited_kmh(70.0),
            "be-vlg:urban" => MaxSpeed::new_limited_kmh(50.0),
            "be-wal:rural" => MaxSpeed::new_limited_kmh(90.0),
            "be-wal:urban" => MaxSpeed::new_limited_kmh(50.0),
            "be:zone30" => MaxSpeed::new_limited_kmh(30.0),
            "ch:motorway" => MaxSpeed::new_limited_kmh(120.0),
            "ch:rural" => MaxSpeed::new_limited_kmh(80.0),
            "ch:trunk" => MaxSpeed::new_limited_kmh(100.0),
            "ch:urban" => MaxSpeed::new_limited_kmh(50.0),
            "cz:living_street" => MaxSpeed::new_limited_kmh(20.0),
            "cz:motorway" => MaxSpeed::new_limited_kmh(130.0),
            "cz:pedestrian_zone" => MaxSpeed::new_limited_kmh(20.0),
            "cz:rural" => MaxSpeed::new_limited_kmh(90.0),
            "cz:trunk" => MaxSpeed::new_limited_kmh(110.0),
            "cz:urban_motorway" => MaxSpeed::new_limited_kmh(80.0),
            "cz:urban" => MaxSpeed::new_limited_kmh(50.0),
            "cz:urban_trunk" => MaxSpeed::new_limited_kmh(80.0),
            "de:bicycle_road" => MaxSpeed::new_limited_kmh(30.0),
            "de:living_street" => MaxSpeed::new_limited_kmh(7.0),
            "de:motorway" => MaxSpeed::new_limited_kmh(130.0),
            "de:rural" => MaxSpeed::new_limited_kmh(100.0),
            "de:urban" => MaxSpeed::new_limited_kmh(50.0),
            "dk:motorway" => MaxSpeed::new_limited_kmh(130.0),
            "dk:rural" => MaxSpeed::new_limited_kmh(80.0),
            "dk:urban" => MaxSpeed::new_limited_kmh(50.0),
            "fr:motorway" => MaxSpeed::new_limited_kmh(130.0), // 130 / 110 (raining)
            "fr:rural" => MaxSpeed::new_limited_kmh(80.0),
            "fr:urban" => MaxSpeed::new_limited_kmh(50.0),
            "fr:zone30" => MaxSpeed::new_limited_kmh(30.0),
            "it:motorway" => MaxSpeed::new_limited_kmh(130.0),
            "it:rural" => MaxSpeed::new_limited_kmh(90.0),
            "it:trunk" => MaxSpeed::new_limited_kmh(110.0),
            "it:urban" => MaxSpeed::new_limited_kmh(50.0),
            _ => RE_MAXSPEED
                .captures(s)
                .as_ref()
                .map(capture_to_maxspeed)
                .unwrap_or_default(),
        };
        Ok(ms)
    }
}

#[inline]
fn capture_to_maxspeed(cap: &Captures) -> MaxSpeed {
    cap.name("value")
        .unwrap()
        .as_str()
        .parse::<f32>()
        .ok()
        .map(|value| {
            if let Some(units) = cap.name("units") {
                match units.as_str() {
                    "kmh" | "kph" | "km/h" | "kmph" => MaxSpeed::new_limited_kmh(value),
                    "mph" | "m/h" => Velocity::new::<mile_per_hour>(value).into(),
                    "knots" | "knot" | "kn" => Velocity::new::<knot>(value).into(),
                    "ms" | "m/s" => Velocity::new::<meter_per_second>(value).into(),
                    _ => MaxSpeed::new_limited_kmh(value),
                }
            } else {
                MaxSpeed::new_limited_kmh(value)
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use uom::si::f32::Velocity;
    use uom::si::velocity::{kilometer_per_hour, knot};

    use crate::osm::tags::maxspeed::MaxSpeed;

    #[test]
    fn test_parse_maxspeed() {
        assert_eq!(
            MaxSpeed::from_str("50").unwrap(),
            MaxSpeed::new_limited_kmh(50.0)
        );
        assert_eq!(
            MaxSpeed::from_str("DE:zone:51").unwrap(),
            MaxSpeed::new_limited_kmh(51.0)
        );
        assert_eq!(
            MaxSpeed::from_str("50 kmh").unwrap(),
            MaxSpeed::new_limited_kmh(50.0)
        );
        assert_eq!(
            MaxSpeed::from_str("3kmh").unwrap(),
            MaxSpeed::new_limited_kmh(3.0)
        );
        assert_eq!(MaxSpeed::from_str("none").unwrap(), MaxSpeed::Unlimited);
        assert_eq!(
            MaxSpeed::from_str("DE:urban").unwrap(),
            MaxSpeed::new_limited_kmh(50.0)
        );
        assert_eq!(
            MaxSpeed::from_str("DE:33").unwrap(),
            MaxSpeed::new_limited_kmh(33.0)
        );
        assert_eq!(
            MaxSpeed::from_str("DE:zone:31").unwrap(),
            MaxSpeed::new_limited_kmh(31.0)
        );
        assert_eq!(
            MaxSpeed::from_str("DE:zone31").unwrap(),
            MaxSpeed::new_limited_kmh(31.0)
        );
        assert_eq!(
            MaxSpeed::from_str("5 knots").unwrap(),
            Velocity::new::<knot>(5.0).into()
        );
        assert_eq!(
            MaxSpeed::from_str("20 mph")
                .unwrap()
                .velocity()
                .unwrap()
                .floor::<kilometer_per_hour>(),
            Velocity::new::<kilometer_per_hour>(32.0)
        );
        assert_eq!(
            MaxSpeed::from_str("walk").unwrap(),
            MaxSpeed::new_limited_kmh(5.0)
        );
    }
}
