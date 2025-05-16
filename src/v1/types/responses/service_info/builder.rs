//! Builders for service information.

use chrono::DateTime;
use chrono::Utc;
use url::Url;

use crate::v1::types::responses::ServiceInfo;
use crate::v1::types::responses::service_info::Artifact;
use crate::v1::types::responses::service_info::Organization;
use crate::v1::types::responses::service_info::ServiceType;
use crate::v1::types::responses::service_info::TES_VERSION;

/// The default group to use for the service.
pub const DEFAULT_GROUP: &str = "org.ga4gh";

/// An error related to a [`Builder`].
#[derive(Copy, Clone, Debug)]
pub enum Error {
    /// A required value was missing for a builder field.
    Missing(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(field) => write!(
                f,
                "missing required value for '{field}' in a service information builder"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for a [`ServiceInfo`].
#[derive(Default)]
pub struct Builder {
    /// The unique identifier for this service.
    ///
    /// Reverse domain name notation is recommended though not required. The
    /// identifier should attempt to be globally unique.
    id: Option<String>,

    /// The human readable name of the service.
    name: Option<String>,

    /// The TES API specification version this service supports.
    tes_version: Option<String>,

    /// A description of the service.
    ///
    /// This should be human readable.
    description: Option<String>,

    /// The name of the organization running this service.
    org_name: Option<String>,

    /// A URL that describes the organization.
    org_url: Option<Url>,

    /// A contact URL.
    ///
    /// This is generally a `mailto:`.
    contact_url: Option<String>,

    /// A documentation URL.
    documentation_url: Option<Url>,

    /// Timestamp when the service was first available.
    created_at: Option<DateTime<Utc>>,

    /// Timestamp when the service was last updated.
    updated_at: Option<DateTime<Utc>>,

    /// The environment within which the service is running.
    ///
    /// This is usually used to distinguish between production, development, and
    /// test environments.
    environment: Option<String>,

    /// The version of the service itself (the code running the service).
    version: Option<String>,

    /// A list of storage locations supported by the service.
    ///
    /// This does not necessarily have to list _all_ storage locations.
    storage: Option<Vec<String>>,
}

impl Builder {
    /// Sets the identifier for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set identifier for the service.
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    /// Sets the name for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set name for the service.
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Sets the TES version for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set TES version for the service.
    pub fn tes_version(mut self, value: impl Into<String>) -> Self {
        self.tes_version = Some(value.into());
        self
    }

    /// Sets the description for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set description for the service.
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Sets the organization name for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set organization name for the
    /// service.
    pub fn org_name(mut self, value: impl Into<String>) -> Self {
        self.org_name = Some(value.into());
        self
    }

    /// Sets the organization URL for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set organization URL for the
    /// service.
    pub fn org_url(mut self, value: impl Into<Url>) -> Self {
        self.org_url = Some(value.into());
        self
    }

    /// Sets the contact URL for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set contact URL for the service.
    pub fn contact_url(mut self, value: impl Into<String>) -> Self {
        self.contact_url = Some(value.into());
        self
    }

    /// Sets the documentation URL for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set documentation URL for the
    /// service.
    pub fn documentation_url(mut self, value: impl Into<Url>) -> Self {
        self.documentation_url = Some(value.into());
        self
    }

    /// Sets the creation time for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set creation time for the
    /// service.
    pub fn created_at(mut self, value: impl Into<DateTime<Utc>>) -> Self {
        self.created_at = Some(value.into());
        self
    }

    /// Sets the updated time for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set updated time for the service.
    pub fn updated_at(mut self, value: impl Into<DateTime<Utc>>) -> Self {
        self.updated_at = Some(value.into());
        self
    }

    /// Sets the environment for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set environment for the service.
    pub fn environment(mut self, value: impl Into<String>) -> Self {
        self.environment = Some(value.into());
        self
    }

    /// Sets the version for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set version for the service.
    pub fn version(mut self, value: impl Into<String>) -> Self {
        self.version = Some(value.into());
        self
    }

    /// Sets the storage locations for the service.
    ///
    /// # Notes
    ///
    /// This silently overrides any previously set storage locations for the
    /// service.
    pub fn storage(mut self, value: impl Into<Vec<String>>) -> Self {
        self.storage = Some(value.into());
        self
    }

    /// Consumes `self` and attempts to builde a [`ServiceInfo`].
    pub fn try_build(self) -> Result<ServiceInfo> {
        let id = self.id.ok_or(Error::Missing("id"))?;
        let name = self.name.ok_or(Error::Missing("name"))?;

        let ty = ServiceType {
            // NOTE: this value is dictated by the specification.
            group: String::from(DEFAULT_GROUP),
            artifact: Artifact::TaskExecutionService,
            version: self
                .tes_version
                .unwrap_or_else(|| String::from(TES_VERSION)),
        };

        let organization = Organization {
            name: self.org_name.ok_or(Error::Missing("organization name"))?,
            url: self.org_url.ok_or(Error::Missing("organization URL"))?,
        };

        let version = self.version.ok_or(Error::Missing("version"))?;

        Ok(ServiceInfo {
            id,
            name,
            ty,
            description: self.description,
            organization,
            contact_url: self.contact_url,
            documentation_url: self.documentation_url,
            created_at: self.created_at,
            updated_at: self.updated_at,
            environment: self.environment,
            version,
            storage: self.storage,
        })
    }
}
