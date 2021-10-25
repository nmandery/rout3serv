use h3ron_graph::formats::osm::osmpbfreader::Tags;
use regex::{Captures, Regex};
use uom::si::f32::Velocity;
use uom::si::velocity::{kilometer_per_hour, knot, meter_per_second, mile_per_hour};

pub mod car;

macro_rules! kmh {
    ($val:expr) => {
        Velocity::new::<kilometer_per_hour>($val as f32)
    };
}

macro_rules! some_kmh {
    ($val:expr) => {
        Some(kmh!($val))
    };
}

fn infer_max_speed(tags: &Tags, highway_class: &str) -> Velocity {
    let max_kmh = tags
        .get("maxspeed") // most specific limit first
        .map(|value| parse_maxspeed(value.trim().to_lowercase().as_str()))
        .flatten()
        .or_else(|| {
            tags.get("zone:maxspeed") // general limit for the zone
                .map(|value| parse_zone_maxspeed(value.trim().to_lowercase().as_str()))
                .flatten()
        });

    match highway_class {
        // TODO: use this to derive the category
        "motorway" | "motorway_link" | "primary" | "primary_link" => {
            max_kmh.unwrap_or_else(|| kmh!(120))
        }
        "rural" | "tertiary" | "tertiary_link" => max_kmh.unwrap_or_else(|| kmh!(80)),
        "trunk" | "trunk_link" | "secondary" | "secondary_link" => {
            max_kmh.unwrap_or_else(|| kmh!(100))
        }
        "urban" | "road" | "unclassified" => max_kmh.unwrap_or_else(|| kmh!(50)),
        "pedestrian" | "footway" | "path" => max_kmh.unwrap_or_else(|| kmh!(5)),
        "living_street" => max_kmh.unwrap_or_else(|| kmh!(7)),
        "bicycle_road" | "service" | "residential" | "track" => max_kmh.unwrap_or_else(|| kmh!(30)),
        //"track" => max_kmh.unwrap_or(kmh!(20.0)), // mostly non-public agriculture/forestry roads
        _ => kmh!(30), // well, better than 0
    }
}

lazy_static! {
    static ref RE_MAXSPEED: Regex =
        Regex::new(r"^([A-Za-z\-]+:(zone:?)?)?(?P<value>[1-9][0-9]*)(\s*(?P<units>[a-zA-Z/]+))?")
            .unwrap();
}

/// parse the OSM "maxspeed" tag contents
fn parse_maxspeed(maxspeed_str: &str) -> Option<Velocity> {
    match maxspeed_str {
        "walk" => some_kmh!(5),
        "none" => some_kmh!(130),
        "" => None,
        _ => RE_MAXSPEED
            .captures(maxspeed_str)
            .as_ref()
            .map(capture_to_velocity)
            .flatten(),
    }
}

/// based on parts of the "Implicit max speed values", used to
/// parse the contents of the "zone:maxspeed" tag.
/// from  https://wiki.openstreetmap.org/wiki/Key:maxspeed
fn parse_zone_maxspeed(zone_name: &str) -> Option<Velocity> {
    match zone_name {
        "AT:bicycle_road" => some_kmh!(30),
        "AT:motorway" => some_kmh!(130),
        "AT:rural" => some_kmh!(100),
        "AT:trunk" => some_kmh!(100),
        "AT:urban" => some_kmh!(50),
        "BE-BRU:rural" => some_kmh!(70),
        "BE-BRU:urban" => some_kmh!(30),
        "BE:cyclestreet" => some_kmh!(30),
        "BE:living_street" => some_kmh!(20),
        "BE:motorway" => some_kmh!(120),
        "BE:trunk" => some_kmh!(120),
        "BE-VLG:rural" => some_kmh!(70),
        "BE-VLG:urban" => some_kmh!(50),
        "BE-WAL:rural" => some_kmh!(90),
        "BE-WAL:urban" => some_kmh!(50),
        "BE:zone30" => some_kmh!(30),
        "CH:motorway" => some_kmh!(120),
        "CH:rural" => some_kmh!(80),
        "CH:trunk" => some_kmh!(100),
        "CH:urban" => some_kmh!(50),
        "CZ:living_street" => some_kmh!(20),
        "CZ:motorway" => some_kmh!(130),
        "CZ:pedestrian_zone" => some_kmh!(20),
        "CZ:rural" => some_kmh!(90),
        "CZ:trunk" => some_kmh!(110),
        "CZ:urban_motorway" => some_kmh!(80),
        "CZ:urban" => some_kmh!(50),
        "CZ:urban_trunk" => some_kmh!(80),
        "DE:bicycle_road" => some_kmh!(30),
        "DE:living_street" => some_kmh!(7),
        "DE:motorway" => some_kmh!(130),
        "DE:rural" => some_kmh!(100),
        "DE:urban" => some_kmh!(50),
        "DK:motorway" => some_kmh!(130),
        "DK:rural" => some_kmh!(80),
        "DK:urban" => some_kmh!(50),
        "FR:motorway" => some_kmh!(130), // 130 / 110 (raining)
        "FR:rural" => some_kmh!(80),
        "FR:urban" => some_kmh!(50),
        "FR:zone30" => some_kmh!(30),
        "IT:motorway" => some_kmh!(130),
        "IT:rural" => some_kmh!(90),
        "IT:trunk" => some_kmh!(110),
        "IT:urban" => some_kmh!(50),
        _ => parse_maxspeed(zone_name),
    }
}

#[inline]
fn capture_to_velocity(cap: &Captures) -> Option<Velocity> {
    cap.name("value")
        .unwrap()
        .as_str()
        .parse::<f32>()
        .ok()
        .map(|value| {
            if let Some(units) = cap.name("units") {
                match units.as_str().to_lowercase().as_str() {
                    "kmh" | "kph" | "km/h" | "kmph" => kmh!(value),
                    "mph" | "m/h" => Velocity::new::<mile_per_hour>(value),
                    "knots" | "knot" => Velocity::new::<knot>(value),
                    "ms" | "m/s" => Velocity::new::<meter_per_second>(value),
                    _ => kmh!(value),
                }
            } else {
                kmh!(value)
            }
        })
}

#[cfg(test)]
mod tests {
    use uom::si::f32::Velocity;
    use uom::si::velocity::{kilometer_per_hour, knot};

    use crate::osm::{parse_maxspeed, parse_zone_maxspeed};

    #[test]
    fn test_parse_zone_maxspeed() {
        assert_eq!(parse_zone_maxspeed("DE:urban"), some_kmh!(50));
        assert_eq!(parse_zone_maxspeed("DE:33"), some_kmh!(33));
        assert_eq!(parse_zone_maxspeed("DE:zone:31"), some_kmh!(31));
        assert_eq!(parse_zone_maxspeed("DE:zone31"), some_kmh!(31));
    }

    #[test]
    fn test_parse_maxspeed() {
        assert_eq!(parse_maxspeed("50"), some_kmh!(50));
        assert_eq!(parse_maxspeed("DE:zone:51"), some_kmh!(51));
        assert_eq!(parse_maxspeed("50 kmh"), some_kmh!(50));
        assert_eq!(parse_maxspeed("3kmh"), some_kmh!(3));
        assert_eq!(parse_maxspeed("none"), some_kmh!(130));
        assert_eq!(parse_maxspeed("5 knots"), Some(Velocity::new::<knot>(5.0)));
        assert_eq!(
            parse_maxspeed("20 mph").map(|x| x.floor::<kilometer_per_hour>()),
            some_kmh!(32)
        );
        assert_eq!(parse_maxspeed("walk"), some_kmh!(5));
    }
}
