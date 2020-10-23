use cargo_registry::views::krate_publish as u;

/// A builder for constructing a dependency of another crate.
pub struct DependencyBuilder {
    name: String,
    registry: Option<String>,
    explicit_name_in_toml: Option<u::EncodableCrateName>,
    version_req: u::EncodableCrateVersionReq,
}

impl DependencyBuilder {
    /// Create a dependency on the crate with the given name.
    pub fn new(name: &str) -> Self {
        DependencyBuilder {
            name: name.to_string(),
            registry: None,
            explicit_name_in_toml: None,
            version_req: u::EncodableCrateVersionReq(semver::VersionReq::parse(">= 0").unwrap()),
        }
    }

    /// Rename this dependency.
    pub fn rename(mut self, new_name: &str) -> Self {
        self.explicit_name_in_toml = Some(u::EncodableCrateName(new_name.to_string()));
        self
    }

    /// Set an alternative registry for this dependency.
    pub fn registry(mut self, registry: &str) -> Self {
        self.registry = Some(registry.to_string());
        self
    }

    /// Set the version requirement for this dependency.
    ///
    /// # Panics
    ///
    /// Panics if the `version_req` string specified isn't a valid `semver::VersionReq`.
    #[track_caller]
    pub fn version_req(mut self, version_req: &str) -> Self {
        self.version_req = u::EncodableCrateVersionReq(
            semver::VersionReq::parse(version_req)
                .expect("version req isn't a valid semver::VersionReq"),
        );
        self
    }

    /// Consume this builder to create a `u::CrateDependency`. If the dependent crate doesn't
    /// already exist, publishing a crate with this dependency will fail.
    pub fn build(self) -> u::EncodableCrateDependency {
        u::EncodableCrateDependency {
            name: u::EncodableCrateName(self.name),
            optional: false,
            default_features: true,
            features: Vec::new(),
            version_req: self.version_req,
            target: None,
            kind: None,
            explicit_name_in_toml: self.explicit_name_in_toml,
            registry: self.registry,
        }
    }
}
