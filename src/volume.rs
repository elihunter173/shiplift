use hyper::Body;

use crate::{
    builder::VolumeCreateOptions,
    errors::Result,
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
