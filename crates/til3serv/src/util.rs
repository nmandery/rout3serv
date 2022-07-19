use anyhow::Result;

pub trait Validate {
    fn validate(&self) -> Result<()>;
}
