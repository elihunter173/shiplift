use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    iter,
};

use futures_util::{stream::Stream, TryFutureExt};
use hyper::Body;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    errors::{Error, Result},
    rep::ExecDetails,
    tty, Docker,
};

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

#[derive(Serialize, Debug)]
pub struct ExecContainerOptions {
    params: HashMap<&'static str, Vec<String>>,
    params_bool: HashMap<&'static str, bool>,
}

impl ExecContainerOptions {
    /// return a new instance of a builder for options
    pub fn builder() -> ExecContainerOptionsBuilder {
        ExecContainerOptionsBuilder::default()
    }

    /// serialize options as a string. returns None if no options are defined
    pub fn serialize(&self) -> Result<String> {
        let mut body = serde_json::Map::new();

        for (k, v) in &self.params {
            body.insert(
                (*k).to_owned(),
                serde_json::to_value(v).map_err(Error::SerdeJsonError)?,
            );
        }

        for (k, v) in &self.params_bool {
            body.insert(
                (*k).to_owned(),
                serde_json::to_value(v).map_err(Error::SerdeJsonError)?,
            );
        }

        serde_json::to_string(&body).map_err(Error::from)
    }
}

#[derive(Default)]
pub struct ExecContainerOptionsBuilder {
    params: HashMap<&'static str, Vec<String>>,
    params_bool: HashMap<&'static str, bool>,
}

impl ExecContainerOptionsBuilder {
    /// Command to run, as an array of strings
    pub fn cmd(
        &mut self,
        cmds: Vec<&str>,
    ) -> &mut Self {
        for cmd in cmds {
            self.params
                .entry("Cmd")
                .or_insert_with(Vec::new)
                .push(cmd.to_owned());
        }
        self
    }

    /// A list of environment variables in the form "VAR=value"
    pub fn env(
        &mut self,
        envs: Vec<&str>,
    ) -> &mut Self {
        for env in envs {
            self.params
                .entry("Env")
                .or_insert_with(Vec::new)
                .push(env.to_owned());
        }
        self
    }

    /// Attach to stdout of the exec command
    pub fn attach_stdout(
        &mut self,
        stdout: bool,
    ) -> &mut Self {
        self.params_bool.insert("AttachStdout", stdout);
        self
    }

    /// Attach to stderr of the exec command
    pub fn attach_stderr(
        &mut self,
        stderr: bool,
    ) -> &mut Self {
        self.params_bool.insert("AttachStderr", stderr);
        self
    }

    pub fn build(&self) -> ExecContainerOptions {
        ExecContainerOptions {
            params: self.params.clone(),
            params_bool: self.params_bool.clone(),
        }
    }
}

/// Interface for creating volumes
#[derive(Serialize, Debug)]
pub struct ExecResizeOptions {
    params: HashMap<&'static str, Value>,
}

impl ExecResizeOptions {
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
    pub fn builder() -> ExecResizeOptionsBuilder {
        ExecResizeOptionsBuilder::new()
    }
}

#[derive(Default)]
pub struct ExecResizeOptionsBuilder {
    params: HashMap<&'static str, Value>,
}

impl ExecResizeOptionsBuilder {
    pub(crate) fn new() -> Self {
        let params = HashMap::new();
        ExecResizeOptionsBuilder { params }
    }

    pub fn height(
        &mut self,
        height: u64,
    ) -> &mut Self {
        self.params.insert("Name", json!(height));
        self
    }

    pub fn width(
        &mut self,
        width: u64,
    ) -> &mut Self {
        self.params.insert("Name", json!(width));
        self
    }

    pub fn build(&self) -> ExecResizeOptions {
        ExecResizeOptions {
            params: self.params.clone(),
        }
    }
}
