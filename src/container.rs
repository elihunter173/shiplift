use std::{collections::HashMap, hash::Hash, io, iter::Peekable, path::Path, time::Duration};

use futures_util::{
    io::{AsyncRead, AsyncWrite},
    stream::Stream,
    TryFutureExt, TryStreamExt,
};
use hyper::Body;
use mime::Mime;
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::{
    errors::{Error, Result},
    exec::{Exec, ExecContainerOptions},
    form_urlencoded,
    rep::{
        Change, Container as ContainerRep, ContainerCreateInfo, ContainerDetails, Exit, Stats, Top,
    },
    tty::{self, Multiplexer as TtyMultiPlexer},
    Docker,
};

/// Interface for accessing and manipulating a docker container
pub struct Container<'a> {
    docker: &'a Docker,
    id: String,
}

impl<'a> Container<'a> {
    /// Exports an interface exposing operations against a container instance
    pub fn new<S>(
        docker: &'a Docker,
        id: S,
    ) -> Self
    where
        S: Into<String>,
    {
        Container {
            docker,
            id: id.into(),
        }
    }

    /// a getter for the container id
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Inspects the current docker container instance's details
    pub async fn inspect(&self) -> Result<ContainerDetails> {
        self.docker
            .get_json::<ContainerDetails>(&format!("/containers/{}/json", self.id)[..])
            .await
    }

    /// Returns a `top` view of information about the container process
    pub async fn top(
        &self,
        psargs: Option<&str>,
    ) -> Result<Top> {
        let mut path = vec![format!("/containers/{}/top", self.id)];
        if let Some(ref args) = psargs {
            let encoded = form_urlencoded::Serializer::new(String::new())
                .append_pair("ps_args", args)
                .finish();
            path.push(encoded)
        }
        self.docker.get_json(&path.join("?")).await
    }

    /// Returns a stream of logs emitted but the container instance
    pub fn logs(
        &self,
        opts: &LogsOptions,
    ) -> impl Stream<Item = Result<tty::TtyChunk>> + Unpin + 'a {
        let mut path = vec![format!("/containers/{}/logs", self.id)];
        if let Some(query) = opts.serialize() {
            path.push(query)
        }

        let stream = Box::pin(self.docker.stream_get(path.join("?")));

        Box::pin(tty::decode(stream))
    }

    /// Attaches a multiplexed TCP stream to the container that can be used to read Stdout, Stderr and write Stdin.
    async fn attach_raw(&self) -> Result<impl AsyncRead + AsyncWrite + Send + 'a> {
        self.docker
            .stream_post_upgrade(
                format!(
                    "/containers/{}/attach?stream=1&stdout=1&stderr=1&stdin=1",
                    self.id
                ),
                None,
            )
            .await
    }

    /// Attaches a `[TtyMultiplexer]` to the container.
    ///
    /// The `[TtyMultiplexer]` implements Stream for returning Stdout and Stderr chunks. It also implements `[AsyncWrite]` for writing to Stdin.
    ///
    /// The multiplexer can be split into its read and write halves with the `[split](TtyMultiplexer::split)` method
    pub async fn attach(&self) -> Result<TtyMultiPlexer<'a>> {
        let tcp_stream = self.attach_raw().await?;

        Ok(TtyMultiPlexer::new(tcp_stream))
    }

    /// Returns a set of changes made to the container instance
    pub async fn changes(&self) -> Result<Vec<Change>> {
        self.docker
            .get_json::<Vec<Change>>(&format!("/containers/{}/changes", self.id)[..])
            .await
    }

    /// Exports the current docker container into a tarball
    pub fn export(&self) -> impl Stream<Item = Result<Vec<u8>>> + 'a {
        self.docker
            .stream_get(format!("/containers/{}/export", self.id))
            .map_ok(|c| c.to_vec())
    }

    /// Returns a stream of stats specific to this container instance
    pub fn stats(&'a self) -> impl Stream<Item = Result<Stats>> + Unpin + 'a {
        let codec = futures_codec::LinesCodec {};

        let reader = Box::pin(
            self.docker
                .stream_get(format!("/containers/{}/stats", self.id))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
        )
        .into_async_read();

        Box::pin(
            futures_codec::FramedRead::new(reader, codec)
                .map_err(Error::IO)
                .and_then(|s: String| async move {
                    serde_json::from_str(&s).map_err(Error::SerdeJsonError)
                }),
        )
    }

    /// Start the container instance
    pub async fn start(&self) -> Result<()> {
        self.docker
            .post(&format!("/containers/{}/start", self.id)[..], None)
            .await?;
        Ok(())
    }

    /// Stop the container instance
    pub async fn stop(
        &self,
        wait: Option<Duration>,
    ) -> Result<()> {
        let mut path = vec![format!("/containers/{}/stop", self.id)];
        if let Some(w) = wait {
            let encoded = form_urlencoded::Serializer::new(String::new())
                .append_pair("t", &w.as_secs().to_string())
                .finish();

            path.push(encoded)
        }
        self.docker.post(&path.join("?"), None).await?;
        Ok(())
    }

    /// Restart the container instance
    pub async fn restart(
        &self,
        wait: Option<Duration>,
    ) -> Result<()> {
        let mut path = vec![format!("/containers/{}/restart", self.id)];
        if let Some(w) = wait {
            let encoded = form_urlencoded::Serializer::new(String::new())
                .append_pair("t", &w.as_secs().to_string())
                .finish();
            path.push(encoded)
        }
        self.docker.post(&path.join("?"), None).await?;
        Ok(())
    }

    /// Kill the container instance
    pub async fn kill(
        &self,
        signal: Option<&str>,
    ) -> Result<()> {
        let mut path = vec![format!("/containers/{}/kill", self.id)];
        if let Some(sig) = signal {
            let encoded = form_urlencoded::Serializer::new(String::new())
                .append_pair("signal", &sig.to_owned())
                .finish();
            path.push(encoded)
        }
        self.docker.post(&path.join("?"), None).await?;
        Ok(())
    }

    /// Rename the container instance
    pub async fn rename(
        &self,
        name: &str,
    ) -> Result<()> {
        let query = form_urlencoded::Serializer::new(String::new())
            .append_pair("name", name)
            .finish();
        self.docker
            .post(
                &format!("/containers/{}/rename?{}", self.id, query)[..],
                None,
            )
            .await?;
        Ok(())
    }

    /// Pause the container instance
    pub async fn pause(&self) -> Result<()> {
        self.docker
            .post(&format!("/containers/{}/pause", self.id)[..], None)
            .await?;
        Ok(())
    }

    /// Unpause the container instance
    pub async fn unpause(&self) -> Result<()> {
        self.docker
            .post(&format!("/containers/{}/unpause", self.id)[..], None)
            .await?;
        Ok(())
    }

    /// Wait until the container stops
    pub async fn wait(&self) -> Result<Exit> {
        self.docker
            .post_json(
                format!("/containers/{}/wait", self.id),
                Option::<(Body, Mime)>::None,
            )
            .await
    }

    /// Delete the container instance
    ///
    /// Use remove instead to use the force/v options.
    pub async fn delete(&self) -> Result<()> {
        self.docker
            .delete(&format!("/containers/{}", self.id)[..])
            .await?;
        Ok(())
    }

    /// Delete the container instance (todo: force/v)
    pub async fn remove(
        &self,
        opts: RmContainerOptions,
    ) -> Result<()> {
        let mut path = vec![format!("/containers/{}", self.id)];
        if let Some(query) = opts.serialize() {
            path.push(query)
        }
        self.docker.delete(&path.join("?")).await?;
        Ok(())
    }

    /// Execute a command in this container
    pub fn exec(
        &'a self,
        opts: &'a ExecContainerOptions,
    ) -> impl Stream<Item = Result<tty::TtyChunk>> + Unpin + 'a {
        Box::pin(
            async move {
                let id = Exec::create_id(&self.docker, &self.id, opts).await?;
                Ok(Exec::_start(&self.docker, &id))
            }
            .try_flatten_stream(),
        )
    }

    /// Copy a file/folder from the container.  The resulting stream is a tarball of the extracted
    /// files.
    ///
    /// If `path` is not an absolute path, it is relative to the container’s root directory. The
    /// resource specified by `path` must exist. To assert that the resource is expected to be a
    /// directory, `path` should end in `/` or `/`. (assuming a path separator of `/`). If `path`
    /// ends in `/.`  then this indicates that only the contents of the path directory should be
    /// copied.  A symlink is always resolved to its target.
    pub fn copy_from(
        &self,
        path: &Path,
    ) -> impl Stream<Item = Result<Vec<u8>>> + 'a {
        let path_arg = form_urlencoded::Serializer::new(String::new())
            .append_pair("path", &path.to_string_lossy())
            .finish();

        let endpoint = format!("/containers/{}/archive?{}", self.id, path_arg);
        self.docker.stream_get(endpoint).map_ok(|c| c.to_vec())
    }

    /// Copy a byte slice as file into (see `bytes`) the container.
    ///
    /// The file will be copied at the given location (see `path`) and will be owned by root
    /// with access mask 644.
    pub async fn copy_file_into<P: AsRef<Path>>(
        &self,
        path: P,
        bytes: &[u8],
    ) -> Result<()> {
        let path = path.as_ref();

        let mut ar = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o0644);
        ar.append_data(
            &mut header,
            path.to_path_buf()
                .iter()
                .skip(1)
                .collect::<std::path::PathBuf>(),
            bytes,
        )
        .unwrap();
        let data = ar.into_inner().unwrap();

        self.copy_to(Path::new("/"), data.into()).await?;
        Ok(())
    }

    /// Copy a tarball (see `body`) to the container.
    ///
    /// The tarball will be copied to the container and extracted at the given location (see `path`).
    pub async fn copy_to(
        &self,
        path: &Path,
        body: Body,
    ) -> Result<()> {
        let path_arg = form_urlencoded::Serializer::new(String::new())
            .append_pair("path", &path.to_string_lossy())
            .finish();

        let mime = "application/x-tar".parse::<Mime>().unwrap();

        self.docker
            .put(
                &format!("/containers/{}/archive?{}", self.id, path_arg),
                Some((body, mime)),
            )
            .await?;
        Ok(())
    }
}

/// Interface for docker containers
pub struct Containers<'a> {
    docker: &'a Docker,
}

impl<'a> Containers<'a> {
    /// Exports an interface for interacting with docker containers
    pub fn new(docker: &'a Docker) -> Containers<'a> {
        Containers { docker }
    }

    /// Lists the container instances on the docker host
    pub async fn list(
        &self,
        opts: &ContainerListOptions,
    ) -> Result<Vec<ContainerRep>> {
        let mut path = vec!["/containers/json".to_owned()];
        if let Some(query) = opts.serialize() {
            path.push(query)
        }
        self.docker
            .get_json::<Vec<ContainerRep>>(&path.join("?"))
            .await
    }

    /// Returns a reference to a set of operations available to a specific container instance
    pub fn get<S>(
        &self,
        name: S,
    ) -> Container
    where
        S: Into<String>,
    {
        Container::new(self.docker, name)
    }

    /// Returns a builder interface for creating a new container instance
    pub async fn create(
        &self,
        opts: &ContainerOptions,
    ) -> Result<ContainerCreateInfo> {
        let body: Body = opts.serialize()?.into();
        let mut path = vec!["/containers/create".to_owned()];

        if let Some(ref name) = opts.name {
            path.push(
                form_urlencoded::Serializer::new(String::new())
                    .append_pair("name", name)
                    .finish(),
            );
        }

        self.docker
            .post_json(&path.join("?"), Some((body, mime::APPLICATION_JSON)))
            .await
    }
}

/// Options for filtering container list results
#[derive(Default, Debug)]
pub struct ContainerListOptions {
    params: HashMap<&'static str, String>,
}

impl ContainerListOptions {
    /// return a new instance of a builder for options
    pub fn builder() -> ContainerListOptionsBuilder {
        ContainerListOptionsBuilder::default()
    }

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

/// Filter options for container listings
pub enum ContainerFilter {
    ExitCode(u64),
    Status(String),
    LabelName(String),
    Label(String, String),
}

/// Builder interface for `ContainerListOptions`
#[derive(Default)]
pub struct ContainerListOptionsBuilder {
    params: HashMap<&'static str, String>,
}

impl ContainerListOptionsBuilder {
    pub fn filter(
        &mut self,
        filters: Vec<ContainerFilter>,
    ) -> &mut Self {
        let mut param = HashMap::new();
        for f in filters {
            match f {
                ContainerFilter::ExitCode(c) => param.insert("exit", vec![c.to_string()]),
                ContainerFilter::Status(s) => param.insert("status", vec![s]),
                ContainerFilter::LabelName(n) => param.insert("label", vec![n]),
                ContainerFilter::Label(n, v) => param.insert("label", vec![format!("{}={}", n, v)]),
            };
        }
        // structure is a a json encoded object mapping string keys to a list
        // of string values
        self.params
            .insert("filters", serde_json::to_string(&param).unwrap());
        self
    }

    pub fn all(&mut self) -> &mut Self {
        self.params.insert("all", "true".to_owned());
        self
    }

    pub fn since(
        &mut self,
        since: &str,
    ) -> &mut Self {
        self.params.insert("since", since.to_owned());
        self
    }

    pub fn before(
        &mut self,
        before: &str,
    ) -> &mut Self {
        self.params.insert("before", before.to_owned());
        self
    }

    pub fn sized(&mut self) -> &mut Self {
        self.params.insert("size", "true".to_owned());
        self
    }

    pub fn build(&self) -> ContainerListOptions {
        ContainerListOptions {
            params: self.params.clone(),
        }
    }
}

/// Interface for building a new docker container from an existing image
#[derive(Serialize, Debug)]
pub struct ContainerOptions {
    pub name: Option<String>,
    params: HashMap<&'static str, Value>,
}

/// Function to insert a JSON value into a tree where the desired
/// location of the value is given as a path of JSON keys.
fn insert<'a, I, V>(
    key_path: &mut Peekable<I>,
    value: &V,
    parent_node: &mut Value,
) where
    V: Serialize,
    I: Iterator<Item = &'a str>,
{
    let local_key = key_path.next().unwrap();

    if key_path.peek().is_some() {
        let node = parent_node
            .as_object_mut()
            .unwrap()
            .entry(local_key.to_string())
            .or_insert(Value::Object(Map::new()));

        insert(key_path, value, node);
    } else {
        parent_node
            .as_object_mut()
            .unwrap()
            .insert(local_key.to_string(), serde_json::to_value(value).unwrap());
    }
}

impl ContainerOptions {
    /// return a new instance of a builder for options
    pub fn builder(name: &str) -> ContainerOptionsBuilder {
        ContainerOptionsBuilder::new(name)
    }

    /// serialize options as a string. returns None if no options are defined
    pub fn serialize(&self) -> Result<String> {
        serde_json::to_string(&self.to_json()).map_err(Error::from)
    }

    fn to_json(&self) -> Value {
        let mut body_members = Map::new();
        // The HostConfig element gets initialized to an empty object,
        // for backward compatibility.
        body_members.insert("HostConfig".to_string(), Value::Object(Map::new()));
        let mut body = Value::Object(body_members);
        self.parse_from(&self.params, &mut body);
        body
    }

    pub fn parse_from<'a, K, V>(
        &self,
        params: &'a HashMap<K, V>,
        body: &mut Value,
    ) where
        &'a HashMap<K, V>: IntoIterator,
        K: ToString + Eq + Hash,
        V: Serialize,
    {
        for (k, v) in params.iter() {
            let key_string = k.to_string();
            insert(&mut key_string.split('.').peekable(), v, body)
        }
    }
}

#[derive(Default)]
pub struct ContainerOptionsBuilder {
    name: Option<String>,
    params: HashMap<&'static str, Value>,
}

impl ContainerOptionsBuilder {
    pub(crate) fn new(image: &str) -> Self {
        let mut params = HashMap::new();

        params.insert("Image", Value::String(image.to_owned()));
        ContainerOptionsBuilder { name: None, params }
    }

    pub fn name(
        &mut self,
        name: &str,
    ) -> &mut Self {
        self.name = Some(name.to_owned());
        self
    }

    /// Specify the working dir (corresponds to the `-w` docker cli argument)
    pub fn working_dir(
        &mut self,
        working_dir: &str,
    ) -> &mut Self {
        self.params.insert("WorkingDir", json!(working_dir));
        self
    }

    /// Specify any bind mounts, taking the form of `/some/host/path:/some/container/path`
    pub fn volumes(
        &mut self,
        volumes: Vec<&str>,
    ) -> &mut Self {
        self.params.insert("HostConfig.Binds", json!(volumes));
        self
    }

    /// enable all exposed ports on the container to be mapped to random, available, ports on the host
    pub fn publish_all_ports(&mut self) -> &mut Self {
        self.params
            .insert("HostConfig.PublishAllPorts", json!(true));
        self
    }

    pub fn expose(
        &mut self,
        srcport: u32,
        protocol: &str,
        hostport: u32,
    ) -> &mut Self {
        let mut exposedport: HashMap<String, String> = HashMap::new();
        exposedport.insert("HostPort".to_string(), hostport.to_string());

        /* The idea here is to go thought the 'old' port binds
         * and to apply them to the local 'port_bindings' variable,
         * add the bind we want and replace the 'old' value */
        let mut port_bindings: HashMap<String, Value> = HashMap::new();
        for (key, val) in self
            .params
            .get("HostConfig.PortBindings")
            .unwrap_or(&json!(null))
            .as_object()
            .unwrap_or(&Map::new())
            .iter()
        {
            port_bindings.insert(key.to_string(), json!(val));
        }
        port_bindings.insert(
            format!("{}/{}", srcport, protocol),
            json!(vec![exposedport]),
        );

        self.params
            .insert("HostConfig.PortBindings", json!(port_bindings));

        // Replicate the port bindings over to the exposed ports config
        let mut exposed_ports: HashMap<String, Value> = HashMap::new();
        let empty_config: HashMap<String, Value> = HashMap::new();
        for key in port_bindings.keys() {
            exposed_ports.insert(key.to_string(), json!(empty_config));
        }

        self.params.insert("ExposedPorts", json!(exposed_ports));

        self
    }

    /// Publish a port in the container without assigning a port on the host
    pub fn publish(
        &mut self,
        srcport: u32,
        protocol: &str,
    ) -> &mut Self {
        /* The idea here is to go thought the 'old' port binds
         * and to apply them to the local 'exposedport_bindings' variable,
         * add the bind we want and replace the 'old' value */
        let mut exposed_port_bindings: HashMap<String, Value> = HashMap::new();
        for (key, val) in self
            .params
            .get("ExposedPorts")
            .unwrap_or(&json!(null))
            .as_object()
            .unwrap_or(&Map::new())
            .iter()
        {
            exposed_port_bindings.insert(key.to_string(), json!(val));
        }
        exposed_port_bindings.insert(format!("{}/{}", srcport, protocol), json!({}));

        // Replicate the port bindings over to the exposed ports config
        let mut exposed_ports: HashMap<String, Value> = HashMap::new();
        let empty_config: HashMap<String, Value> = HashMap::new();
        for key in exposed_port_bindings.keys() {
            exposed_ports.insert(key.to_string(), json!(empty_config));
        }

        self.params.insert("ExposedPorts", json!(exposed_ports));

        self
    }

    pub fn links(
        &mut self,
        links: Vec<&str>,
    ) -> &mut Self {
        self.params.insert("HostConfig.Links", json!(links));
        self
    }

    pub fn memory(
        &mut self,
        memory: u64,
    ) -> &mut Self {
        self.params.insert("HostConfig.Memory", json!(memory));
        self
    }

    /// Total memory limit (memory + swap) in bytes. Set to -1 (default) to enable unlimited swap.
    pub fn memory_swap(
        &mut self,
        memory_swap: i64,
    ) -> &mut Self {
        self.params
            .insert("HostConfig.MemorySwap", json!(memory_swap));
        self
    }

    /// CPU quota in units of 10<sup>-9</sup> CPUs. Set to 0 (default) for there to be no limit.
    ///
    /// For example, setting `nano_cpus` to `500_000_000` results in the container being allocated
    /// 50% of a single CPU, while `2_000_000_000` results in the container being allocated 2 CPUs.
    pub fn nano_cpus(
        &mut self,
        nano_cpus: u64,
    ) -> &mut Self {
        self.params.insert("HostConfig.NanoCpus", json!(nano_cpus));
        self
    }

    /// CPU quota in units of CPUs. This is a wrapper around `nano_cpus` to do the unit conversion.
    ///
    /// See [`nano_cpus`](#method.nano_cpus).
    pub fn cpus(
        &mut self,
        cpus: f64,
    ) -> &mut Self {
        self.nano_cpus((1_000_000_000.0 * cpus) as u64)
    }

    /// Sets an integer value representing the container's relative CPU weight versus other
    /// containers.
    pub fn cpu_shares(
        &mut self,
        cpu_shares: u32,
    ) -> &mut Self {
        self.params
            .insert("HostConfig.CpuShares", json!(cpu_shares));
        self
    }

    pub fn labels(
        &mut self,
        labels: &HashMap<&str, &str>,
    ) -> &mut Self {
        self.params.insert("Labels", json!(labels));
        self
    }

    /// Whether to attach to `stdin`.
    pub fn attach_stdin(
        &mut self,
        attach: bool,
    ) -> &mut Self {
        self.params.insert("AttachStdin", json!(attach));
        self.params.insert("OpenStdin", json!(attach));
        self
    }

    /// Whether to attach to `stdout`.
    pub fn attach_stdout(
        &mut self,
        attach: bool,
    ) -> &mut Self {
        self.params.insert("AttachStdout", json!(attach));
        self
    }

    /// Whether to attach to `stderr`.
    pub fn attach_stderr(
        &mut self,
        attach: bool,
    ) -> &mut Self {
        self.params.insert("AttachStderr", json!(attach));
        self
    }

    /// Whether standard streams should be attached to a TTY.
    pub fn tty(
        &mut self,
        tty: bool,
    ) -> &mut Self {
        self.params.insert("Tty", json!(tty));
        self
    }

    pub fn extra_hosts(
        &mut self,
        hosts: Vec<&str>,
    ) -> &mut Self {
        self.params.insert("HostConfig.ExtraHosts", json!(hosts));
        self
    }

    pub fn volumes_from(
        &mut self,
        volumes: Vec<&str>,
    ) -> &mut Self {
        self.params.insert("HostConfig.VolumesFrom", json!(volumes));
        self
    }

    pub fn network_mode(
        &mut self,
        network: &str,
    ) -> &mut Self {
        self.params.insert("HostConfig.NetworkMode", json!(network));
        self
    }

    pub fn env<E, S>(
        &mut self,
        envs: E,
    ) -> &mut Self
    where
        S: AsRef<str> + Serialize,
        E: AsRef<[S]> + Serialize,
    {
        self.params.insert("Env", json!(envs));
        self
    }

    pub fn cmd(
        &mut self,
        cmds: Vec<&str>,
    ) -> &mut Self {
        self.params.insert("Cmd", json!(cmds));
        self
    }

    pub fn entrypoint(
        &mut self,
        entrypoint: &str,
    ) -> &mut Self {
        self.params.insert("Entrypoint", json!(entrypoint));
        self
    }

    pub fn capabilities(
        &mut self,
        capabilities: Vec<&str>,
    ) -> &mut Self {
        self.params.insert("HostConfig.CapAdd", json!(capabilities));
        self
    }

    pub fn devices(
        &mut self,
        devices: Vec<HashMap<String, String>>,
    ) -> &mut Self {
        self.params.insert("HostConfig.Devices", json!(devices));
        self
    }

    pub fn log_driver(
        &mut self,
        log_driver: &str,
    ) -> &mut Self {
        self.params
            .insert("HostConfig.LogConfig.Type", json!(log_driver));
        self
    }

    pub fn restart_policy(
        &mut self,
        name: &str,
        maximum_retry_count: u64,
    ) -> &mut Self {
        self.params
            .insert("HostConfig.RestartPolicy.Name", json!(name));
        if name == "on-failure" {
            self.params.insert(
                "HostConfig.RestartPolicy.MaximumRetryCount",
                json!(maximum_retry_count),
            );
        }
        self
    }

    pub fn auto_remove(
        &mut self,
        set: bool,
    ) -> &mut Self {
        self.params.insert("HostConfig.AutoRemove", json!(set));
        self
    }

    /// Signal to stop a container as a string. Default is "SIGTERM".
    pub fn stop_signal(
        &mut self,
        sig: &str,
    ) -> &mut Self {
        self.params.insert("StopSignal", json!(sig));
        self
    }

    /// Signal to stop a container as an integer. Default is 15 (SIGTERM).
    pub fn stop_signal_num(
        &mut self,
        sig: u64,
    ) -> &mut Self {
        self.params.insert("StopSignal", json!(sig));
        self
    }

    /// Timeout to stop a container. Only seconds are counted. Default is 10s
    pub fn stop_timeout(
        &mut self,
        timeout: Duration,
    ) -> &mut Self {
        self.params.insert("StopTimeout", json!(timeout.as_secs()));
        self
    }

    pub fn userns_mode(
        &mut self,
        mode: &str,
    ) -> &mut Self {
        self.params.insert("HostConfig.UsernsMode", json!(mode));
        self
    }

    pub fn privileged(
        &mut self,
        set: bool,
    ) -> &mut Self {
        self.params.insert("HostConfig.Privileged", json!(set));
        self
    }

    pub fn user(
        &mut self,
        user: &str,
    ) -> &mut Self {
        self.params.insert("User", json!(user));
        self
    }

    pub fn build(&self) -> ContainerOptions {
        ContainerOptions {
            name: self.name.clone(),
            params: self.params.clone(),
        }
    }
}

/// Options for controlling log request results
#[derive(Default, Debug)]
pub struct LogsOptions {
    params: HashMap<&'static str, String>,
}

impl LogsOptions {
    /// return a new instance of a builder for options
    pub fn builder() -> LogsOptionsBuilder {
        LogsOptionsBuilder::default()
    }

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

/// Builder interface for `LogsOptions`
#[derive(Default)]
pub struct LogsOptionsBuilder {
    params: HashMap<&'static str, String>,
}

impl LogsOptionsBuilder {
    pub fn follow(
        &mut self,
        f: bool,
    ) -> &mut Self {
        self.params.insert("follow", f.to_string());
        self
    }

    pub fn stdout(
        &mut self,
        s: bool,
    ) -> &mut Self {
        self.params.insert("stdout", s.to_string());
        self
    }

    pub fn stderr(
        &mut self,
        s: bool,
    ) -> &mut Self {
        self.params.insert("stderr", s.to_string());
        self
    }

    pub fn timestamps(
        &mut self,
        t: bool,
    ) -> &mut Self {
        self.params.insert("timestamps", t.to_string());
        self
    }

    /// how_many can either be "all" or a to_string() of the number
    pub fn tail(
        &mut self,
        how_many: &str,
    ) -> &mut Self {
        self.params.insert("tail", how_many.to_owned());
        self
    }

    #[cfg(feature = "chrono")]
    pub fn since<Tz>(
        &mut self,
        timestamp: &chrono::DateTime<Tz>,
    ) -> &mut Self
    where
        Tz: chrono::TimeZone,
    {
        self.params
            .insert("since", timestamp.timestamp().to_string());
        self
    }

    #[cfg(not(feature = "chrono"))]
    pub fn since(
        &mut self,
        timestamp: i64,
    ) -> &mut Self {
        self.params.insert("since", timestamp.to_string());
        self
    }

    pub fn build(&self) -> LogsOptions {
        LogsOptions {
            params: self.params.clone(),
        }
    }
}

/// Options for controlling log request results
#[derive(Default, Debug)]
pub struct RmContainerOptions {
    params: HashMap<&'static str, String>,
}

impl RmContainerOptions {
    /// return a new instance of a builder for options
    pub fn builder() -> RmContainerOptionsBuilder {
        RmContainerOptionsBuilder::default()
    }

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

/// Builder interface for `LogsOptions`
#[derive(Default)]
pub struct RmContainerOptionsBuilder {
    params: HashMap<&'static str, String>,
}

impl RmContainerOptionsBuilder {
    pub fn force(
        &mut self,
        f: bool,
    ) -> &mut Self {
        self.params.insert("force", f.to_string());
        self
    }

    pub fn volumes(
        &mut self,
        s: bool,
    ) -> &mut Self {
        self.params.insert("v", s.to_string());
        self
    }

    pub fn build(&self) -> RmContainerOptions {
        RmContainerOptions {
            params: self.params.clone(),
        }
    }
}
