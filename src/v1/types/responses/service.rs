//! Responses related to the service itself.

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

/// Names of specifications supported.
///
/// Note that, in the case of the Task Execution Service specification, this can
/// only be `"tes"` but it's still technically listed as an enum.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Artifact {
    /// A task execution service.
    #[serde(rename = "tes")]
    #[default]
    TaskExecutionService,
}

/// An organization provided a TES service.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Organization {
    /// The organization name.
    pub name: String,

    /// A URL for the organization.
    pub url: Url,
}

/// A type of service.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServiceType {
    /// Namespace in reverse domain name format.
    pub group: String,

    /// Name of the specification implemented.
    pub artifact: Artifact,

    /// The version of the specification being implemented.
    pub version: String,
}

/// A set of service information for the server.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    /// A unique identifier for the service.
    pub id: String,

    /// Human-readable name of the service.
    pub name: String,

    /// The type of the service.
    pub r#type: ServiceType,

    /// An optional description of the service.
    pub description: Option<String>,

    /// The organization running the service.
    pub organization: Organization,

    /// An optional contact URL.
    pub contact_url: Option<String>,

    /// An optional documentation URL.
    pub documentation_url: Option<Url>,

    /// Timestamp when the service was first available.
    pub created_at: Option<DateTime<Utc>>,

    /// Timestamp when the service was last updated.
    pub updated_at: Option<DateTime<Utc>>,

    /// An optional string describing the environment that the service is
    /// running within.
    pub environment: Option<String>,

    /// The version of the service.
    pub version: String,

    /// Lists some, but not necessarily all, storage locations supported by the
    /// service.
    pub storage: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

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
        assert_eq!(result.r#type.group, "org.ga4gh");
        assert_eq!(result.r#type.artifact, Artifact::TaskExecutionService);
        assert_eq!(result.r#type.version, "1.0.0");
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

    #[test]
    fn full_conversion() {
        let now = DateTime::parse_from_rfc3339("2024-09-07T20:27:35.345673Z")
            .unwrap()
            .into();

        let info = ServiceInfo {
            id: String::from("org.ga4gh.myservice"),
            name: String::from("My Server"),
            r#type: ServiceType {
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
