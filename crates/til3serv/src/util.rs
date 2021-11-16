use eyre::Result;

pub trait Validate {
    fn validate(&self) -> Result<()>;
}
