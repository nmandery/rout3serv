//!
//! Reference at https://wiki.openstreetmap.org/wiki/Key:sidewalk

use h3ron_graph::io::osm::osmpbfreader::Tags;

use crate::osm::tags::str_to_bool;

pub struct SideWalk {
    pub on_left_side: bool,
    pub on_left_bicycles_allowed: bool,
    pub on_right_side: bool,
    pub on_right_bicycles_allowed: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for SideWalk {
    fn default() -> Self {
        Self {
            on_left_side: false,
            on_left_bicycles_allowed: false,
            on_right_side: false,
            on_right_bicycles_allowed: false,
        }
    }
}

impl SideWalk {
    fn is_without_sidewalk(&self) -> bool {
        !(self.on_left_side || self.on_right_side)
    }
}

pub fn infer_sidewalk(tags: &Tags) -> Option<SideWalk> {
    let mut sidewalk = SideWalk::default();

    for (tag_key, tag_value) in tags.iter() {
        let tag_key = tag_key.to_lowercase();
        let tag_value = tag_value.to_lowercase();
        match tag_key.trim() {
            "sidewalk" => match tag_value.trim() {
                "both" | "yes" => {
                    sidewalk.on_right_side = true;
                    sidewalk.on_left_side = true;
                }
                "left" => sidewalk.on_left_side = true,
                "right" => sidewalk.on_right_side = true,
                _ => {}
            },
            "sidewalk:both:bicycle" => {
                sidewalk.on_right_bicycles_allowed = str_to_bool(&tag_value).unwrap_or(false);
                sidewalk.on_left_bicycles_allowed = sidewalk.on_right_bicycles_allowed;
            }
            "sidewalk:left:bicycle" => {
                sidewalk.on_left_bicycles_allowed = str_to_bool(&tag_value).unwrap_or(false);
            }
            "sidewalk:right:bicycle" => {
                sidewalk.on_left_bicycles_allowed = str_to_bool(&tag_value).unwrap_or(false);
            }
            _ => {}
        }
    }

    if sidewalk.is_without_sidewalk() {
        None
    } else {
        Some(sidewalk)
    }
}
