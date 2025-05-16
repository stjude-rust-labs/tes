//! Responses related to the service itself.

use chrono::DateTime;
use chrono::Utc;
use url::Url;

mod builder;

pub use builder::Builder;

/// The TES version implemented.
pub const TES_VERSION: &str = "1.1.0";

/// Names of specifications supported.
///
/// Note that, in the case of the Task Execution Service specification, this can
/// only be `"tes"` but it's still technically listed as an enum.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Artifact {
    /// A task execution service.
    #[cfg_attr(feature = "serde", serde(rename = "tes"))]
    #[default]
    TaskExecutionService,
}

/// An organization provided a TES service.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Organization {
    /// The organization name.
    pub name: String,

    /// A URL for the organization.
    pub url: Url,
}

/// A type of service.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ServiceType {
    /// Namespace in reverse domain name format.
    pub group: String,

    /// Name of the specification implemented.
    pub artifact: Artifact,

    /// The version of the specification being implemented.
    pub version: String,
}

/// A set of service information for the server.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ServiceInfo {
    /// A unique identifier for the service.
    id: String,

    /// Human-readable name of the service.
    name: String,

    /// The type of the service.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    ty: ServiceType,

    /// An optional description of the service.
    description: Option<String>,

    /// The organization running the service.
    organization: Organization,

    /// An optional contact URL.
    contact_url: Option<String>,

    /// An optional documentation URL.
    documentation_url: Option<Url>,

    /// Timestamp when the service was first available.
    created_at: Option<DateTime<Utc>>,

    /// Timestamp when the service was last updated.
    updated_at: Option<DateTime<Utc>>,

    /// An optional string describing the environment that the service is
    /// running within.
    environment: Option<String>,

    /// The version of the service.
    version: String,

    /// Lists some, but not necessarily all, storage locations supported by the
    /// service.
    storage: Option<Vec<String>>,
}

impl ServiceInfo {
    /// Gets the identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Gets the name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the service type.
    pub fn ty(&self) -> &ServiceType {
        &self.ty
    }

    /// Gets the description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Gets the organization.
    pub fn organization(&self) -> &Organization {
        &self.organization
    }

    /// Gets the contact URL.
    pub fn contact_url(&self) -> Option<&str> {
        self.contact_url.as_deref()
    }

    /// Gets the documentation URL.
    pub fn documentation_url(&self) -> Option<&Url> {
        self.documentation_url.as_ref()
    }

    /// Gets the created at time.
    pub fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    /// Gets the updated at time.
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }

    /// Gets the environment.
    pub fn environment(&self) -> Option<&String> {
        self.environment.as_ref()
    }

    /// Gets the service version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Gets the storage locations.
    pub fn storage(&self) -> Option<&Vec<String>> {
        self.storage.as_ref()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "serde")]
    use pretty_assertions::assert_eq;

    #[cfg(feature = "serde")]
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn smoke() {
        let content = r#"{
  "id": "org.ga4gh.myservice",
  "name": "My project",
  "type": {
    "group": "org.ga4gh",
    "artifact": "tes",
    "version": "1.0.0"
  },
  "description": "This service provides...",
  "organization": {
    "name": "My organization",
    "url": "https://example.com"
  },
  "contactUrl": "mailto:support@example.com",
  "documentationUrl": "https://docs.myservice.example.com",
  "createdAt": "2019-06-04T12:58:19Z",
  "updatedAt": "2019-06-04T12:58:19Z",
  "environment": "test",
  "version": "1.0.0",
  "storage": [
    "file:///path/to/local/funnel-storage",
    "s3://ohsu-compbio-funnel/storage"
  ]
}"#;

        let result: ServiceInfo = serde_json::from_str(content).unwrap();

        assert_eq!(result.id, "org.ga4gh.myservice");
        assert_eq!(result.name, "My project");
        assert_eq!(result.ty.group, "org.ga4gh");
        assert_eq!(result.ty.artifact, Artifact::TaskExecutionService);
        assert_eq!(result.ty.version, "1.0.0");
        assert_eq!(result.description.unwrap(), "This service provides...");
        assert_eq!(result.organization.name, "My organization");
        assert_eq!(result.organization.url.to_string(), "https://example.com/");
        assert_eq!(result.contact_url.unwrap(), "mailto:support@example.com");
        assert_eq!(
            result.documentation_url.unwrap().to_string(),
            "https://docs.myservice.example.com/"
        );
        assert_eq!(
            result.created_at.unwrap().to_rfc3339(),
            "2019-06-04T12:58:19+00:00"
        );
        assert_eq!(
            result.updated_at.unwrap().to_rfc3339(),
            "2019-06-04T12:58:19+00:00"
        );
        assert_eq!(result.environment.unwrap(), "test");
        assert_eq!(result.version, "1.0.0");
        assert_eq!(
            result.storage.unwrap(),
            vec![
                "file:///path/to/local/funnel-storage",
                "s3://ohsu-compbio-funnel/storage"
            ]
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn full_conversion() {
        let now = DateTime::parse_from_rfc3339("2024-09-07T20:27:35.345673Z")
            .unwrap()
            .into();

        let info = ServiceInfo {
            id: String::from("org.ga4gh.myservice"),
            name: String::from("My Server"),
            ty: ServiceType {
                group: String::from("org.ga4gh"),
                artifact: Artifact::TaskExecutionService,
                version: String::from("1.0.0"),
            },
            description: Some(String::from("A description")),
            organization: Organization {
                name: String::from("My Organization"),
                url: Url::try_from("https://example.com").unwrap(),
            },
            contact_url: Some(String::from("mailto:foo@bar.com")),
            documentation_url: Some(Url::try_from("https://docs.myservice.example.com").unwrap()),
            created_at: Some(now),
            updated_at: Some(now),
            environment: Some(String::from("test")),
            version: String::from("1.5.0"),
            storage: Some(vec![
                String::from("file:///path/to/local/funnel-storage"),
                String::from("s3://ohsu-compbio-funnel/storage"),
            ]),
        };

        let serialized = serde_json::to_string(&info).unwrap();
        assert_eq!(
            serialized,
            r#"{"id":"org.ga4gh.myservice","name":"My Server","type":{"group":"org.ga4gh","artifact":"tes","version":"1.0.0"},"description":"A description","organization":{"name":"My Organization","url":"https://example.com/"},"contactUrl":"mailto:foo@bar.com","documentationUrl":"https://docs.myservice.example.com/","createdAt":"2024-09-07T20:27:35.345673Z","updatedAt":"2024-09-07T20:27:35.345673Z","environment":"test","version":"1.5.0","storage":["file:///path/to/local/funnel-storage","s3://ohsu-compbio-funnel/storage"]}"#
        );

        let deserialized: ServiceInfo = serde_json::from_str(&serialized).unwrap();
        assert_eq!(info, deserialized);
    }
}
