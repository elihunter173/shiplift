//! Interfaces for building various structures

#[cfg(test)]
mod tests {
    use super::{ContainerOptionsBuilder, LogsOptionsBuilder, RegistryAuth};

    #[test]
    fn container_options_simple() {
        let builder = ContainerOptionsBuilder::new("test_image");
        let options = builder.build();

        assert_eq!(
            r#"{"HostConfig":{},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    #[test]
    fn container_options_env() {
        let options = ContainerOptionsBuilder::new("test_image")
            .env(vec!["foo", "bar"])
            .build();

        assert_eq!(
            r#"{"Env":["foo","bar"],"HostConfig":{},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    #[test]
    fn container_options_env_dynamic() {
        let env: Vec<String> = ["foo", "bar", "baz"]
            .iter()
            .map(|s| String::from(*s))
            .collect();

        let options = ContainerOptionsBuilder::new("test_image").env(&env).build();

        assert_eq!(
            r#"{"Env":["foo","bar","baz"],"HostConfig":{},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    #[test]
    fn container_options_user() {
        let options = ContainerOptionsBuilder::new("test_image")
            .user("alice")
            .build();

        assert_eq!(
            r#"{"HostConfig":{},"Image":"test_image","User":"alice"}"#,
            options.serialize().unwrap()
        );
    }

    #[test]
    fn container_options_host_config() {
        let options = ContainerOptionsBuilder::new("test_image")
            .network_mode("host")
            .auto_remove(true)
            .privileged(true)
            .build();

        assert_eq!(
            r#"{"HostConfig":{"AutoRemove":true,"NetworkMode":"host","Privileged":true},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    #[test]
    fn container_options_expose() {
        let options = ContainerOptionsBuilder::new("test_image")
            .expose(80, "tcp", 8080)
            .build();
        assert_eq!(
            r#"{"ExposedPorts":{"80/tcp":{}},"HostConfig":{"PortBindings":{"80/tcp":[{"HostPort":"8080"}]}},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
        // try exposing two
        let options = ContainerOptionsBuilder::new("test_image")
            .expose(80, "tcp", 8080)
            .expose(81, "tcp", 8081)
            .build();
        assert_eq!(
            r#"{"ExposedPorts":{"80/tcp":{},"81/tcp":{}},"HostConfig":{"PortBindings":{"80/tcp":[{"HostPort":"8080"}],"81/tcp":[{"HostPort":"8081"}]}},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    #[test]
    fn container_options_publish() {
        let options = ContainerOptionsBuilder::new("test_image")
            .publish(80, "tcp")
            .build();
        assert_eq!(
            r#"{"ExposedPorts":{"80/tcp":{}},"HostConfig":{},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
        // try exposing two
        let options = ContainerOptionsBuilder::new("test_image")
            .publish(80, "tcp")
            .publish(81, "tcp")
            .build();
        assert_eq!(
            r#"{"ExposedPorts":{"80/tcp":{},"81/tcp":{}},"HostConfig":{},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    /// Test container option PublishAllPorts
    #[test]
    fn container_options_publish_all_ports() {
        let options = ContainerOptionsBuilder::new("test_image")
            .publish_all_ports()
            .build();

        assert_eq!(
            r#"{"HostConfig":{"PublishAllPorts":true},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    /// Test container options that are nested 3 levels deep.
    #[test]
    fn container_options_nested() {
        let options = ContainerOptionsBuilder::new("test_image")
            .log_driver("fluentd")
            .build();

        assert_eq!(
            r#"{"HostConfig":{"LogConfig":{"Type":"fluentd"}},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    /// Test the restart policy settings
    #[test]
    fn container_options_restart_policy() {
        let mut options = ContainerOptionsBuilder::new("test_image")
            .restart_policy("on-failure", 10)
            .build();

        assert_eq!(
            r#"{"HostConfig":{"RestartPolicy":{"MaximumRetryCount":10,"Name":"on-failure"}},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );

        options = ContainerOptionsBuilder::new("test_image")
            .restart_policy("always", 0)
            .build();

        assert_eq!(
            r#"{"HostConfig":{"RestartPolicy":{"Name":"always"}},"Image":"test_image"}"#,
            options.serialize().unwrap()
        );
    }

    /// Test registry auth with token
    #[test]
    fn registry_auth_token() {
        let options = RegistryAuth::token("abc");
        assert_eq!(
            base64::encode(r#"{"identitytoken":"abc"}"#),
            options.serialize()
        );
    }

    /// Test registry auth with username and password
    #[test]
    fn registry_auth_password_simple() {
        let options = RegistryAuth::builder()
            .username("user_abc")
            .password("password_abc")
            .build();
        assert_eq!(
            base64::encode(r#"{"username":"user_abc","password":"password_abc"}"#),
            options.serialize()
        );
    }

    /// Test registry auth with all fields
    #[test]
    fn registry_auth_password_all() {
        let options = RegistryAuth::builder()
            .username("user_abc")
            .password("password_abc")
            .email("email_abc")
            .server_address("https://example.org")
            .build();
        assert_eq!(
            base64::encode(
                r#"{"username":"user_abc","password":"password_abc","email":"email_abc","serveraddress":"https://example.org"}"#
            ),
            options.serialize()
        );
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn logs_options() {
        let timestamp = chrono::NaiveDateTime::from_timestamp(2_147_483_647, 0);
        let since = chrono::DateTime::<chrono::Utc>::from_utc(timestamp, chrono::Utc);

        let options = LogsOptionsBuilder::default()
            .follow(true)
            .stdout(true)
            .stderr(true)
            .timestamps(true)
            .tail("all")
            .since(&since)
            .build();

        let serialized = options.serialize().unwrap();

        assert!(serialized.contains("follow=true"));
        assert!(serialized.contains("stdout=true"));
        assert!(serialized.contains("stderr=true"));
        assert!(serialized.contains("timestamps=true"));
        assert!(serialized.contains("tail=all"));
        assert!(serialized.contains("since=2147483647"));
    }

    #[cfg(not(feature = "chrono"))]
    #[test]
    fn logs_options() {
        let options = LogsOptionsBuilder::default()
            .follow(true)
            .stdout(true)
            .stderr(true)
            .timestamps(true)
            .tail("all")
            .since(2_147_483_647)
            .build();

        let serialized = options.serialize().unwrap();

        assert!(serialized.contains("follow=true"));
        assert!(serialized.contains("stdout=true"));
        assert!(serialized.contains("stderr=true"));
        assert!(serialized.contains("timestamps=true"));
        assert!(serialized.contains("tail=all"));
        assert!(serialized.contains("since=2147483647"));
    }
}
