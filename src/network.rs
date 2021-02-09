use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

use hyper::Body;
use serde::Serialize;
use serde_json::{json, Value};
use url::form_urlencoded;

use crate::rep::{NetworkCreateInfo, NetworkDetails as NetworkInfo};
use crate::{
    errors::{Error, Result},
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

/// Options for filtering networks list results
#[derive(Default, Debug)]
pub struct NetworkListOptions {
    params: HashMap<&'static str, String>,
}

impl NetworkListOptions {
    /// serialize options as a string. returns None if no options are defined
    pub fn serialize(&self) -> Option<String> {
        if self.params.is_empty() {
            None
        } else {
            Some(
                form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(&self.params)
                    .finish(),
            )
        }
    }
}

/// Interface for creating new docker network
#[derive(Serialize, Debug)]
pub struct NetworkCreateOptions {
    params: HashMap<&'static str, Value>,
}

impl NetworkCreateOptions {
    /// return a new instance of a builder for options
    pub fn builder(name: &str) -> NetworkCreateOptionsBuilder {
        NetworkCreateOptionsBuilder::new(name)
    }

    /// serialize options as a string. returns None if no options are defined
    pub fn serialize(&self) -> Result<String> {
        serde_json::to_string(&self.params).map_err(Error::from)
    }

    pub fn parse_from<'a, K, V>(
        &self,
        params: &'a HashMap<K, V>,
        body: &mut serde_json::Map<String, Value>,
    ) where
        &'a HashMap<K, V>: IntoIterator,
        K: ToString + Eq + Hash,
        V: Serialize,
    {
        for (k, v) in params.iter() {
            let key = k.to_string();
            let value = serde_json::to_value(v).unwrap();

            body.insert(key, value);
        }
    }
}

#[derive(Default)]
pub struct NetworkCreateOptionsBuilder {
    params: HashMap<&'static str, Value>,
}

impl NetworkCreateOptionsBuilder {
    pub(crate) fn new(name: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("Name", json!(name));
        NetworkCreateOptionsBuilder { params }
    }

    pub fn driver(
        &mut self,
        name: &str,
    ) -> &mut Self {
        if !name.is_empty() {
            self.params.insert("Driver", json!(name));
        }
        self
    }

    pub fn label(
        &mut self,
        labels: HashMap<String, String>,
    ) -> &mut Self {
        self.params.insert("Labels", json!(labels));
        self
    }

    pub fn build(&self) -> NetworkCreateOptions {
        NetworkCreateOptions {
            params: self.params.clone(),
        }
    }
}

/// Interface for connect container to network
#[derive(Serialize, Debug)]
pub struct ContainerConnectionOptions {
    params: HashMap<&'static str, Value>,
}

impl ContainerConnectionOptions {
    /// serialize options as a string. returns None if no options are defined
    pub fn serialize(&self) -> Result<String> {
        serde_json::to_string(&self.params).map_err(Error::from)
    }

    pub fn parse_from<'a, K, V>(
        &self,
        params: &'a HashMap<K, V>,
        body: &mut BTreeMap<String, Value>,
    ) where
        &'a HashMap<K, V>: IntoIterator,
        K: ToString + Eq + Hash,
        V: Serialize,
    {
        for (k, v) in params.iter() {
            let key = k.to_string();
            let value = serde_json::to_value(v).unwrap();

            body.insert(key, value);
        }
    }

    /// return a new instance of a builder for options
    pub fn builder(container_id: &str) -> ContainerConnectionOptionsBuilder {
        ContainerConnectionOptionsBuilder::new(container_id)
    }
}

#[derive(Default)]
pub struct ContainerConnectionOptionsBuilder {
    params: HashMap<&'static str, Value>,
}

impl ContainerConnectionOptionsBuilder {
    pub(crate) fn new(container_id: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("Container", json!(container_id));
        ContainerConnectionOptionsBuilder { params }
    }

    pub fn aliases(
        &mut self,
        aliases: Vec<&str>,
    ) -> &mut Self {
        self.params
            .insert("EndpointConfig", json!({ "Aliases": json!(aliases) }));
        self
    }

    pub fn force(&mut self) -> &mut Self {
        self.params.insert("Force", json!(true));
        self
    }

    pub fn build(&self) -> ContainerConnectionOptions {
        ContainerConnectionOptions {
            params: self.params.clone(),
        }
    }
}
