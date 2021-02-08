use std::iter;

use futures_util::{stream::Stream, TryFutureExt};
use hyper::Body;

use crate::builder::{ExecContainerOptions, ExecResizeOptions};
use crate::{rep::ExecDetails, tty, Docker, errors::Result};

/// Interface for docker exec instance
pub struct Exec<'a> {
    docker: &'a Docker,
    id: String,
}

impl<'a> Exec<'a> {
    fn new<S>(
        docker: &'a Docker,
        id: S,
    ) -> Exec<'a>
    where
        S: Into<String>,
    {
        Exec {
            docker,
            id: id.into(),
        }
    }

    /// Creates an exec instance in docker and returns its id
    pub(crate) async fn create_id(
        docker: &'a Docker,
        container_id: &str,
        opts: &ExecContainerOptions,
    ) -> Result<String> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Response {
            id: String,
        }

        let body: Body = opts.serialize()?.into();

        docker
            .post_json(
                &format!("/containers/{}/exec", container_id)[..],
                Some((body, mime::APPLICATION_JSON)),
            )
            .await
            .map(|resp: Response| resp.id)
    }

    /// Starts an exec instance with id exec_id
    pub(crate) fn _start(
        docker: &'a Docker,
        exec_id: &str,
    ) -> impl Stream<Item = Result<tty::TtyChunk>> + 'a {
        let bytes: &[u8] = b"{}";

        let stream = Box::pin(docker.stream_post(
            format!("/exec/{}/start", &exec_id),
            Some((bytes.into(), mime::APPLICATION_JSON)),
            None::<iter::Empty<_>>,
        ));

        tty::decode(stream)
    }

    /// Creates a new exec instance that will be executed in a container with id == container_id
    pub async fn create(
        docker: &'a Docker,
        container_id: &str,
        opts: &ExecContainerOptions,
    ) -> Result<Exec<'a>> {
        Ok(Exec::new(
            docker,
            Exec::create_id(docker, container_id, opts).await?,
        ))
    }

    /// Get a reference to a set of operations available to an already created exec instance.
    ///
    /// It's in callers responsibility to ensure that exec instance with specified id actually
    /// exists. Use [Exec::create](Exec::create) to ensure that the exec instance is created
    /// beforehand.
    pub async fn get<S>(
        docker: &'a Docker,
        id: S,
    ) -> Exec<'a>
    where
        S: Into<String>,
    {
        Exec::new(docker, id)
    }

    /// Starts this exec instance returning a multiplexed tty stream
    pub fn start(&'a self) -> impl Stream<Item = Result<tty::TtyChunk>> + 'a {
        Box::pin(
            async move {
                let bytes: &[u8] = b"{}";

                let stream = Box::pin(self.docker.stream_post(
                    format!("/exec/{}/start", &self.id),
                    Some((bytes.into(), mime::APPLICATION_JSON)),
                    None::<iter::Empty<_>>,
                ));

                Ok(tty::decode(stream))
            }
            .try_flatten_stream(),
        )
    }

    /// Inspect this exec instance to aquire detailed information
    pub async fn inspect(&self) -> Result<ExecDetails> {
        self.docker
            .get_json(&format!("/exec/{}/json", &self.id)[..])
            .await
    }

    pub async fn resize(
        &self,
        opts: &ExecResizeOptions,
    ) -> Result<()> {
        let body: Body = opts.serialize()?.into();

        self.docker
            .post_json(
                &format!("/exec/{}/resize", &self.id)[..],
                Some((body, mime::APPLICATION_JSON)),
            )
            .await
    }
}
