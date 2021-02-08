use hyper::Body;

use crate::rep::{NetworkCreateInfo, NetworkDetails as NetworkInfo};
use crate::{
    builder::{ContainerConnectionOptions, NetworkCreateOptions, NetworkListOptions},
    errors::Result,
    Docker,
};

/// Interface for docker network
pub struct Networks<'a> {
    docker: &'a Docker,
}

impl<'a> Networks<'a> {
    /// Exports an interface for interacting with docker Networks
    pub fn new(docker: &Docker) -> Networks {
        Networks { docker }
    }

    /// List the docker networks on the current docker host
    pub async fn list(
        &self,
        opts: &NetworkListOptions,
    ) -> Result<Vec<NetworkInfo>> {
        let mut path = vec!["/networks".to_owned()];
        if let Some(query) = opts.serialize() {
            path.push(query);
        }
        self.docker.get_json(&path.join("?")).await
    }

    /// Returns a reference to a set of operations available to a specific network instance
    pub fn get<S>(
        &self,
        id: S,
    ) -> Network
    where
        S: Into<String>,
    {
        Network::new(self.docker, id)
    }

    /// Create a new Network instance
    pub async fn create(
        &self,
        opts: &NetworkCreateOptions,
    ) -> Result<NetworkCreateInfo> {
        let body: Body = opts.serialize()?.into();
        let path = vec!["/networks/create".to_owned()];

        self.docker
            .post_json(&path.join("?"), Some((body, mime::APPLICATION_JSON)))
            .await
    }
}

/// Interface for accessing and manipulating a docker network
pub struct Network<'a> {
    docker: &'a Docker,
    id: String,
}

impl<'a> Network<'a> {
    /// Exports an interface exposing operations against a network instance
    pub fn new<S>(
        docker: &Docker,
        id: S,
    ) -> Network
    where
        S: Into<String>,
    {
        Network {
            docker,
            id: id.into(),
        }
    }

    /// a getter for the Network id
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Inspects the current docker network instance's details
    pub async fn inspect(&self) -> Result<NetworkInfo> {
        self.docker
            .get_json(&format!("/networks/{}", self.id)[..])
            .await
    }

    /// Delete the network instance
    pub async fn delete(&self) -> Result<()> {
        self.docker
            .delete(&format!("/networks/{}", self.id)[..])
            .await?;
        Ok(())
    }

    /// Connect container to network
    pub async fn connect(
        &self,
        opts: &ContainerConnectionOptions,
    ) -> Result<()> {
        self.do_connection("connect", opts).await
    }

    /// Disconnect container to network
    pub async fn disconnect(
        &self,
        opts: &ContainerConnectionOptions,
    ) -> Result<()> {
        self.do_connection("disconnect", opts).await
    }

    async fn do_connection(
        &self,
        segment: &str,
        opts: &ContainerConnectionOptions,
    ) -> Result<()> {
        let body: Body = opts.serialize()?.into();

        self.docker
            .post(
                &format!("/networks/{}/{}", self.id, segment)[..],
                Some((body, mime::APPLICATION_JSON)),
            )
            .await?;
        Ok(())
    }
}
