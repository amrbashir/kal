#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("Couldn't find $HOME directory")]
    HomeDirNotFound,
}

pub type Result<T> = std::result::Result<T, Error>;
