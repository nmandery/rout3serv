use h3o::Resolution;
use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub struct GraphKey {
    pub name: String,
    pub h3_resolution: Resolution,
}

impl GraphKey {
    pub fn file_suffix() -> &'static str {
        ".ipc"
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
        match RE_GRAPH_FILE.captures(s) {
            Some(cap) => {
                let h3_resolution = Resolution::try_from(
                    cap.name("h3_res")
                        .unwrap()
                        .as_str()
                        .parse::<u8>()
                        .map_err(|_| crate::io::Error::NotAGraphKey)?,
                )
                .map_err(|_| crate::io::Error::NotAGraphKey)?;
                Ok(Self {
                    name: cap.name("name").unwrap().as_str().to_string(),
                    h3_resolution,
                })
            }
            None => Err(crate::io::Error::NotAGraphKey),
        }
    }
}

impl ToString for GraphKey {
    fn to_string(&self) -> String {
        format!(
            "{}_{}{}",
            self.name,
            u8::from(self.h3_resolution),
            GraphKey::file_suffix()
        )
    }
}

#[cfg(test)]
mod tests {
    use h3o::Resolution;
    use std::str::FromStr;

    use crate::io::GraphKey;

    #[test]
    fn graph_regex() {
        assert_eq!(
            GraphKey::from_str("somegraph_7.ipc").unwrap(),
            GraphKey {
                name: "somegraph".to_string(),
                h3_resolution: Resolution::Seven,
            }
        );
    }
}
