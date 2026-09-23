//! A client for interacting with a Task Execution Service (TES) service.

use std::fmt;
use std::time::Duration;

use reqwest::Method;
pub use reqwest::RequestBuilder;
use reqwest::Response;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use tokio_retry2::Retry;
use tokio_retry2::RetryError;
use tracing::debug;
use tracing::trace;
use tracing::warn;
use url::Url;

use crate::auth::Authorizer;
use crate::v1::types::requests;
use crate::v1::types::requests::DEFAULT_PAGE_SIZE;
use crate::v1::types::requests::GetTaskParams;
use crate::v1::types::requests::ListTasksParams;
use crate::v1::types::requests::MAX_PAGE_SIZE;
use crate::v1::types::requests::View;
use crate::v1::types::responses;
use crate::v1::types::responses::CreatedTask;
use crate::v1::types::responses::ListTasks;
use crate::v1::types::responses::MinimalTask;
use crate::v1::types::responses::ServiceInfo;
use crate::v1::types::responses::TaskResponse;

mod builder;

pub use builder::Builder;
// Re-export the strategy module so users can easily pass in retry strategies.
pub use tokio_retry2::strategy;

/// Helper for notifying that a network operation failed and will be retried.
fn notify_retry(e: &impl fmt::Display, duration: Duration) {
    // Duration of 0 indicates the first attempt; only print the message for a
    // retry
    if !duration.is_zero() {
        let secs = duration.as_secs();
        warn!(
            "network operation failed (retried after waiting {secs} second{s}): {e}",
            s = if secs == 1 { "" } else { "s" }
        );
    }
}

/// An error within the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An invalid request was made.
    #[error("{0}")]
    InvalidRequest(String),
    /// An error when serializing or deserializing JSON.
    #[error(transparent)]
    SerdeJSON(#[from] serde_json::Error),
    /// An error when serializing or deserializing JSON.
    #[error(transparent)]
    SerdeParams(#[from] serde_url_params::Error),
    /// An error from `reqwest`.
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    /// An error occurred during authorization.
    #[error("authorization with the TES service failed: {0}")]
    Authorization(crate::auth::Error),
}

/// A [`Result`](std::result::Result) with an [`Error`].
type Result<T> = std::result::Result<T, Error>;

/// A client for interacting with a service.
pub struct Client {
    /// The base URL.
    url: Url,
    /// The underlying client.
    client: reqwest::Client,
    /// The authorizer to use for requests.
    authorizer: Option<Box<dyn Authorizer>>,
}

impl Client {
    /// Gets an empty builder for a [`Client`].
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Performs a `GET` request on an endpoint within the service.
    ///
    /// # Safety
    ///
    /// Because calls to `get()` are all local to this crate, the provided
    /// `endpoint` is assumed to always be joinable to the base URL without
    /// issue.
    async fn get<T>(
        &self,
        endpoint: impl AsRef<str>,
        retries: impl IntoIterator<Item = Duration>,
    ) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let endpoint = endpoint.as_ref();

        // SAFETY: as described in the documentation for this method, the URL is
        // already validated upon creating of the [`Client`], and the
        // `endpoint` is assumed to always be joinable to that URL, so this
        // should always unwrap.
        let url = self.url.join(endpoint).unwrap();
        debug!("GET {url}");

        let bytes = Retry::spawn_notify(
            retries,
            || async {
                let response = self
                    .send_request(|| self.client.request(Method::GET, url.clone()))
                    .await?;

                response
                    .bytes()
                    .await
                    .map_err(|e| RetryError::transient(Error::Reqwest(e)))
            },
            notify_retry,
        )
        .await?;

        trace!("{bytes:?}");
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Performs a `POST1` request on an endpoint within the service.
    ///
    /// # Safety
    ///
    /// Because calls to `post()` are all local to this crate, the provided
    /// `endpoint` is assumed to always be joinable to the base URL without
    /// issue.
    async fn post<T>(
        &self,
        endpoint: impl AsRef<str>,
        body: impl Serialize,
        retries: impl IntoIterator<Item = Duration>,
    ) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let endpoint = endpoint.as_ref();
        let body = serde_json::to_string(&body)?;

        // SAFETY: as described in the documentation for this method, the URL is
        // already validated upon creation of the [`Client`], and the
        // `endpoint` is assumed to always be joinable to that URL, so this
        // should always unwrap.
        let url = self.url.join(endpoint).unwrap();
        debug!("POST {url} {body}");

        let resp = Retry::spawn_notify(
            retries,
            || async {
                let response = self
                    .send_request(|| {
                        self.client
                            .request(Method::POST, url.clone())
                            .body(body.clone())
                            .header("Content-Type", "application/json")
                    })
                    .await?;

                response
                    .json::<T>()
                    .await
                    .map_err(|e| RetryError::transient(Error::Reqwest(e)))
            },
            notify_retry,
        )
        .await?;

        Ok(resp)
    }

    /// Gets the service information.
    ///
    /// The provided `retries` iterator is the number of durations to wait
    /// between retries; an empty iterator implies no retries.
    ///
    /// This method makes a request to the `GET /service-info` endpoint.
    pub async fn service_info(
        &self,
        retries: impl IntoIterator<Item = Duration>,
    ) -> Result<ServiceInfo> {
        self.get("service-info", retries).await
    }

    /// Lists tasks within the service.
    ///
    /// The provided `retries` iterator is the number of durations to wait
    /// between retries; an empty iterator implies no retries.
    ///
    /// This method makes a request to the `GET /tasks` endpoint.
    pub async fn list_tasks(
        &self,
        params: Option<&ListTasksParams>,
        retries: impl IntoIterator<Item = Duration>,
    ) -> Result<ListTasks<TaskResponse>> {
        if let Some(params) = params
            && params.page_size.unwrap_or(DEFAULT_PAGE_SIZE) >= MAX_PAGE_SIZE
        {
            return Err(Error::InvalidRequest(format!(
                "page size must be less than {MAX_PAGE_SIZE}"
            )));
        }

        let url = match params {
            Some(params) => format!(
                "tasks?{params}",
                params = serde_url_params::to_string(params)?
            ),
            None => "tasks".to_string(),
        };

        match params.and_then(|p| p.view).unwrap_or_default() {
            View::Minimal => {
                let results = self.get::<ListTasks<MinimalTask>>(url, retries).await?;

                Ok(ListTasks {
                    next_page_token: results.next_page_token,
                    tasks: results
                        .tasks
                        .into_iter()
                        .map(TaskResponse::Minimal)
                        .collect::<Vec<_>>(),
                })
            }
            View::Basic => {
                let results = self.get::<ListTasks<responses::Task>>(url, retries).await?;

                Ok(ListTasks {
                    next_page_token: results.next_page_token,
                    tasks: results
                        .tasks
                        .into_iter()
                        .map(TaskResponse::Basic)
                        .collect::<Vec<_>>(),
                })
            }
            View::Full => {
                let results = self.get::<ListTasks<responses::Task>>(url, retries).await?;

                Ok(ListTasks {
                    next_page_token: results.next_page_token,
                    tasks: results
                        .tasks
                        .into_iter()
                        .map(TaskResponse::Full)
                        .collect::<Vec<_>>(),
                })
            }
        }
    }

    /// Creates a task within the service.
    ///
    /// The provided `retries` iterator is the number of durations to wait
    /// between retries; an empty iterator implies no retries.
    ///
    /// This method makes a request to the `POST /tasks` endpoint.
    pub async fn create_task(
        &self,
        task: &requests::Task,
        retries: impl IntoIterator<Item = Duration>,
    ) -> Result<CreatedTask> {
        self.post("tasks", task, retries).await
    }

    /// Gets a specific task within the service.
    ///
    /// The provided `retries` iterator is the number of durations to wait
    /// between retries; an empty iterator implies no retries.
    ///
    /// This method makes a request to the `GET /tasks/{id}` endpoint.
    pub async fn get_task(
        &self,
        id: impl AsRef<str>,
        params: Option<&GetTaskParams>,
        retries: impl IntoIterator<Item = Duration>,
    ) -> Result<TaskResponse> {
        let id = id.as_ref();

        let url = match params {
            Some(params) => format!(
                "tasks/{id}?{params}",
                params = serde_url_params::to_string(params)?
            ),
            None => format!("tasks/{id}"),
        };

        Ok(match params.map(|p| p.view).unwrap_or_default() {
            View::Minimal => TaskResponse::Minimal(self.get(url, retries).await?),
            View::Basic => TaskResponse::Basic(self.get(url, retries).await?),
            View::Full => TaskResponse::Full(self.get(url, retries).await?),
        })
    }

    /// Cancels a task within the service.
    ///
    /// The provided `retries` iterator is the number of durations to wait
    /// between retries; an empty iterator implies no retries.
    ///
    /// This method makes a request to the `POST /tasks/{id}:cancel` endpoint.
    pub async fn cancel_task(
        &self,
        id: impl AsRef<str>,
        retries: impl IntoIterator<Item = Duration>,
    ) -> Result<()> {
        // TES returns an empty JSON object on success
        // See: https://ga4gh.github.io/task-execution-schemas/docs/#tag/TaskService/operation/CancelTask
        let _: serde_json::Value = self
            .post(format!("tasks/{}:cancel", id.as_ref()), (), retries)
            .await?;
        Ok(())
    }

    /// Sends a request given the callback for creating a [`RequestBuilder`].
    ///
    /// This handles reauthorization if the initial request fails with a 401.
    async fn send_request<F>(&self, request: F) -> std::result::Result<Response, RetryError<Error>>
    where
        F: Fn() -> RequestBuilder,
    {
        let mut initial = true;

        loop {
            // Create the request and apply the authorization
            let mut req = request();
            if let Some(authorizer) = &self.authorizer {
                req = authorizer
                    .authorize(initial, req)
                    .await
                    .map_err(Error::Authorization)?;
            }

            // Send the request
            let response = req
                .send()
                .await
                .map_err(|e| RetryError::transient(Error::from(e)))?;

            // Treat server errors as transient
            if response.status().is_server_error() {
                return Err(RetryError::transient(Error::from(
                    response.error_for_status().expect_err("should be error"),
                )));
            }

            // Attempt reauthorization if needed; this occurs at most once
            if initial
                && response.status() == StatusCode::UNAUTHORIZED
                && self
                    .authorizer
                    .as_ref()
                    .map(|a| a.reauthorizes())
                    .unwrap_or(false)
            {
                initial = false;
                continue;
            }

            // Treat other response errors as permanent
            return response
                .error_for_status()
                .map_err(|e| RetryError::permanent(Error::from(e)));
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("url", &self.url)
            .field("client", &self.client)
            .field("authorizer", &"...")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::OAuthAuthorizer;
    use crate::auth::tests::OAuthTestServer;

    #[tokio::test]
    async fn test_authorization() {
        let mut oauth = OAuthTestServer::new(true).await;
        let client = Client::builder()
            .authorizer(OAuthAuthorizer::new(oauth.config.clone(), |_| {}))
            .url(oauth.server.url().parse::<Url>().unwrap())
            .try_build()
            .unwrap();

        let tasks_endpoint = oauth
            .server
            .mock("GET", "/tasks")
            .match_header("authorization", "Bearer ABC")
            .with_header("content-type", "application/json")
            .with_body(r#"{ "tasks": [] }"#)
            .create();
        let response = client.list_tasks(None, std::iter::empty()).await.unwrap();
        assert!(response.tasks.is_empty());

        oauth.assert();
        tasks_endpoint.assert();
    }

    #[tokio::test]
    async fn test_reauthorization() {
        let mut oauth = OAuthTestServer::new(true).await;

        let refresh_endpoint = oauth
            .server
            .mock("POST", "/oauth/token")
            .match_body("grant_type=refresh_token&refresh_token=XYZ&client_id=12345")
            .with_status(200)
            .with_body(
                r#"{ "access_token": "DEF", "refresh_token": "123", "token_type": "Bearer" }"#,
            )
            .create();

        let client = Client::builder()
            .authorizer(OAuthAuthorizer::new(oauth.config.clone(), |_| {}))
            .url(oauth.server.url().parse::<Url>().unwrap())
            .try_build()
            .unwrap();

        let unauthorized = oauth
            .server
            .mock("GET", "/tasks")
            .match_header("authorization", "Bearer ABC")
            .with_status(401)
            .create();
        let authorized = oauth
            .server
            .mock("GET", "/tasks")
            .match_header("authorization", "Bearer DEF")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "tasks": [] }"#)
            .create();
        let response = client.list_tasks(None, std::iter::empty()).await.unwrap();
        assert!(response.tasks.is_empty());

        oauth.assert();
        refresh_endpoint.assert();
        unauthorized.assert();
        authorized.assert();
    }
}
