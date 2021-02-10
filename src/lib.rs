//! Shiplift is a multi-transport utility for maneuvering [docker](https://www.docker.com/) containers
//!
//! # examples
//!
//! ```no_run
//! # async {
//! let docker = shiplift::Docker::new();
//!
//! match docker.images().list(&Default::default()).await {
//!     Ok(images) => {
//!         for image in images {
//!             println!("{:?}", image.repo_tags);
//!         }
//!     },
//!     Err(e) => eprintln!("Something bad happened! {}", e),
//! }
//! # };
//! ```

pub mod errors;
pub mod transport;
pub mod tty;

mod container;
mod docker;
mod exec;
mod image;
mod network;
mod service;
mod volume;

mod tarball;

#[cfg(feature = "chrono")]
mod datetime;

pub use hyper::Uri;

pub use crate::{
    container::{
        Container, ContainerFilter, ContainerListOptions, ContainerOptions, Containers,
        LogsOptions, RmContainerOptions,
    },
    docker::Docker,
    errors::{Error, Result},
    exec::{Exec, ExecContainerOptions, ExecResizeOptions},
    image::{
        BuildOptions, Image, ImageFilter, ImageListOptions, Images, PullOptions, RegistryAuth,
        TagOptions,
    },
    network::{
        ContainerConnectionOptions, Network, NetworkCreateOptions, NetworkListOptions, Networks,
    },
    service::{Service, ServiceListOptions, ServiceOptions, Services},
    transport::Transport,
    volume::{Volume, VolumeCreateOptions, Volumes},
};
