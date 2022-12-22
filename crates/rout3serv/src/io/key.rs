use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub struct GraphKey {
    pub name: String,
    pub h3_resolution: u8,
}

impl GraphKey {
    pub fn file_suffix() -> &'static str {
        ".bincode.zstd"
    }
}

static RE_GRAPH_FILE: Lazy<Regex> = Lazy::new(|| {
    let graph_re_string: String = format!(
        "(?P<name>[a-zA-Z0-9\\-_]+)_(?P<h3_res>[0-9]?[0-9]){}$",
        regex::escape(GraphKey::file_suffix())
    );
    Regex::new(&graph_re_string).unwrap()
});

impl FromStr for GraphKey {
    type Err = crate::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RE_GRAPH_FILE
            .captures(s)
            .map(|cap| Self {
                name: cap.name("name").unwrap().as_str().to_string(),
                h3_resolution: cap.name("h3_res").unwrap().as_str().parse().unwrap(),
            })
            .ok_or(crate::io::Error::NotAGraphKey)
    }
}

impl ToString for GraphKey {
    fn to_string(&self) -> String {
        format!(
            "{}_{}{}",
            self.name,
            self.h3_resolution,
            GraphKey::file_suffix()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::io::GraphKey;

    #[test]
    fn graph_regex() {
        assert_eq!(
            GraphKey::from_str("somegraph_7.bincode.zstd").unwrap(),
            GraphKey {
                name: "somegraph".to_string(),
                h3_resolution: 7,
            }
        );
    }
}
