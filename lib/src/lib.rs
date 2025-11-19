use thiserror::Error;

pub mod ocr;
pub mod theme;
mod util;
pub mod wfinfo;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown Theme")]
    UnknownTheme,
    #[error("Invalid Image Format")]
    InvalidImageFormat,
    #[error(transparent)]
    InitializeError(#[from] tesseract::InitializeError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    TessBaseApiSetImageSafetyError(#[from] tesseract::plumbing::TessBaseApiSetImageSafetyError),
    #[error(transparent)]
    TessBaseApiGetUtf8TextError(#[from] tesseract::plumbing::TessBaseApiGetUtf8TextError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// Re-Exports
pub use image;
pub use palette;
pub use tesseract;
pub use levenshtein;
pub use lazy_static;
pub use serde;
pub use serde_json;
pub use rayon;