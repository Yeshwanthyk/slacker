#![doc = "Library surface for converting GIPHY links into Slack-ready emoji GIFs."]

mod args;
mod convert;
mod error;
pub mod source;

pub use args::{Config, parse};
pub use convert::{Product, make};
pub use error::Error;
