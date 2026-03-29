#[cfg(feature = "with-client")]
pub(crate) mod client;
#[cfg(feature = "with-server")]
mod server;

#[cfg(feature = "with-client")]
pub(crate) use client::DataParser;
#[cfg(feature = "with-server")]
pub(crate) use server::decode;
