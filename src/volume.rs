use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

use hyper::Body;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    errors::{Error, Result},
    rep::{Volume as VolumeRep, VolumeCreateInfo, Volumes as VolumesRep},
    Docker,
};

/// Interface for docker volumes
pub struct Volumes<'a> {
    docker: &'a Docker,
}

impl<'a> Volumes<'a> {
    /// Exports an interface for interacting with docker volumes
    pub fn new(docker: &Docker) -> Volumes {
        Volumes { docker }
    }

    pub async fn create(
        &self,
        opts: &VolumeCreateOptions,
    ) -> Result<VolumeCreateInfo> {
        let body: Body = opts.serialize()?.into();
        let path = vec!["/volumes/create".to_owned()];

        self.docker
            .post_json(&path.join("?"), Some((body, mime::APPLICATION_JSON)))
            .await
    }

    /// Lists the docker volumes on the current docker host
    pub async fn list(&self) -> Result<Vec<VolumeRep>> {
        let path = vec!["/volumes".to_owned()];

        let volumes_rep = self.docker.get_json::<VolumesRep>(&path.join("?")).await?;
        Ok(match volumes_rep.volumes {
            Some(volumes) => volumes,
            None => vec![],
        })
    }

    /// Returns a reference to a set of operations available for a named volume
    pub fn get(
        &self,
        name: &str,
    ) -> Volume {
        Volume::new(self.docker, name)
    }
}

/// Interface for accessing and manipulating a named docker volume
pub struct Volume<'a> {
    docker: &'a Docker,
    name: String,
}

impl<'a> Volume<'a> {
    /// Exports an interface for operations that may be performed against a named volume
    pub fn new<S>(
        docker: &Docker,
        name: S,
    ) -> Volume
    where
        S: Into<String>,
    {
        Volume {
            docker,
            name: name.into(),
        }
    }

    /// Deletes a volume
    pub async fn delete(&self) -> Result<()> {
        self.docker
            .delete(&format!("/volumes/{}", self.name)[..])
            .await?;
        Ok(())
    }
}

/// Interface for creating volumes
#[derive(Serialize, Debug)]
pub struct VolumeCreateOptions {
    params: HashMap<&'static str, Value>,
}

impl VolumeCreateOptions {
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
    pub fn builder() -> VolumeCreateOptionsBuilder {
        VolumeCreateOptionsBuilder::new()
    }
}

#[derive(Default)]
pub struct VolumeCreateOptionsBuilder {
    params: HashMap<&'static str, Value>,
}

impl VolumeCreateOptionsBuilder {
    pub(crate) fn new() -> Self {
        let params = HashMap::new();
        VolumeCreateOptionsBuilder { params }
    }

    pub fn name(
        &mut self,
        name: &str,
    ) -> &mut Self {
        self.params.insert("Name", json!(name));
        self
    }

    pub fn labels(
        &mut self,
        labels: &HashMap<&str, &str>,
    ) -> &mut Self {
        self.params.insert("Labels", json!(labels));
        self
    }

    pub fn build(&self) -> VolumeCreateOptions {
        VolumeCreateOptions {
            params: self.params.clone(),
        }
    }
}
