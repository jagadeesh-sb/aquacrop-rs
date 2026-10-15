use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The inputs cannot be simulated (bad dates, unknown crop, weather too short...).
    Input(String),
    /// The simulation reached a state AquaCrop-OSPy itself rejects (an assertion there).
    Model(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Input(m) | Error::Model(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}
