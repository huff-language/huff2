use clap::ValueEnum;

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
pub enum EvmVersion {
    Paris,
    Shanghai,
    Cancun,
}

impl EvmVersion {
    pub(crate) fn allows_push0(&self) -> bool {
        matches!(self, Self::Shanghai | Self::Cancun)
    }
}
