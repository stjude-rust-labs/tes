//! A client for interacting with a Task Execution Service (TES) service.

use reqwest_middleware::ClientWithMiddleware as ReqwestClient;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::trace;
use url::Url;

use crate::v1::client::tasks::View;
use crate::v1::types::Task;
use crate::v1::types::responses::CreateTask;
use crate::v1::types::responses::ListTasks;
use crate::v1::types::responses::ServiceInfo;
use crate::v1::types::responses::task;
use crate::v1::types::responses::task::MinimalTask;

mod builder;
mod options;
pub mod tasks;

pub use builder::Builder;
pub use options::Options;

/// An error within the client.
#[derive(Debug)]
pub enum Error {
    /// An error when serializing or deserializing JSON.
    SerdeJSON(serde_json::Error),

    /// A middleware error from `reqwest_middleware`.
    // Note: `reqwest_middleware` stores these as an `anyhow::Error` internally.
    Middlware(anyhow::Error),

    /// An error from `reqwest`.
    Reqwest(reqwest::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SerdeJSON(err) => write!(f, "json serde error: {err}"),
            Error::Middlware(err) => write!(f, "middleware error: {err}"),
            Error::Reqwest(err) => write!(f, "reqwest error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
type Result<T> = std::result::Result<T, Error>;

impl From<reqwest_middleware::Error> for Error {
    fn from(value: reqwest_middleware::Error) -> Self {
        match value {
            reqwest_middleware::Error::Middleware(err) => Error::Middlware(err),
            reqwest_middleware::Error::Reqwest(err) => Error::Reqwest(err),
        }
    }
}

/// A client for interacting with a service.
#[derive(Debug)]
pub struct Client {
    /// The base URL.
    url: Url,

    /// The underlying client.
    client: ReqwestClient,
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
    async fn get<Response>(&self, endpoint: impl AsRef<str>) -> Result<Response>
    where
        Response: for<'de> Deserialize<'de>,
    {
        let endpoint = endpoint.as_ref();

        // SAFETY: as described in the documentation for this method, the URL is
        // already validated upon creating of the [`Client`], and the
        // `endpoint` is assumed to always be joinable to that URL, so this
        // should always unwrap.
        let url = self.url.join(endpoint).unwrap();
        debug!("GET {url}");

        let bytes = self
            .client
            .get(url)
            .send()
            .await
            .map_err(Error::from)?
            .bytes()
            .await
            .map_err(Error::Reqwest)?;

        trace!("{bytes:?}");

        serde_json::from_slice(&bytes).map_err(Error::SerdeJSON)
    }

    /// Performs a `POST1` request on an endpoint within the service.
    ///
    /// # Safety
    ///
    /// Because calls to `post()` are all local to this crate, the provided
    /// `endpoint` is assumed to always be joinable to the base URL without
    /// issue.
    async fn post<Body, Response>(&self, endpoint: impl AsRef<str>, body: Body) -> Result<Response>
    where
        Body: Serialize,
        Response: for<'de> Deserialize<'de>,
    {
        let endpoint = endpoint.as_ref();
        let body = serde_json::to_string(&body).map_err(Error::SerdeJSON)?;

        // SAFETY: as described in the documentation for this method, the URL is
        // already validated upon creationg of the [`Client`], and the
        // `endpoint` is assumed to always be joinable to that URL, so this
        // should always unwrap.
        let url = self.url.join(endpoint).unwrap();
        debug!("POST {url} {body}");

        self.client
            .post(url)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(Error::from)?
            .json::<Response>()
            .await
            .map_err(Error::Reqwest)
    }

    /// Gets the service information.
    ///
    /// This method makes a request to the `GET /service-info` endpoint.
    pub async fn service_info(&self) -> Result<ServiceInfo> {
        self.get("service-info").await
    }

    /// Lists a single page of tasks within the service.
    ///
    /// This method makes a request to the `GET /tasks` endpoint.
    pub async fn list_tasks(
        &self,
        view: &View,
        next_token: Option<&str>,
    ) -> Result<ListTasks<task::Response>> {
        let mut url = format!("./tasks?view={view}");

        if let Some(token) = next_token {
            url.push_str("&page_token=");
            url.push_str(token);
        }

        match view {
            View::Minimal => {
                let results = self.get::<ListTasks<MinimalTask>>(url).await?;

                Ok(ListTasks {
                    next_page_token: results.next_page_token,
                    tasks: results
                        .tasks
                        .into_iter()
                        .map(task::Response::Minimal)
                        .collect::<Vec<_>>(),
                })
            }
            View::Basic => {
                let results = self.get::<ListTasks<Task>>(url).await?;

                Ok(ListTasks {
                    next_page_token: results.next_page_token,
                    tasks: results
                        .tasks
                        .into_iter()
                        .map(task::Response::Basic)
                        .collect::<Vec<_>>(),
                })
            }
            View::Full => {
                let results = self.get::<ListTasks<Task>>(url).await?;

                Ok(ListTasks {
                    next_page_token: results.next_page_token,
                    tasks: results
                        .tasks
                        .into_iter()
                        .map(task::Response::Full)
                        .collect::<Vec<_>>(),
                })
            }
        }
    }

    /// Lists all tasks within the service.
    ///
    /// This method is a convenience wrapper around [`Self::list_tasks()`] that
    /// submits follow on requests and the server says there are more results.
    pub async fn list_all_tasks(&self, view: View) -> Result<Vec<task::Response>> {
        let mut results = Vec::new();
        let mut next_token = None;
        let mut page = 1usize;

        loop {
            debug!("reading task page {page} with token {next_token:?}",);

            let response = self.list_tasks(&view, next_token.as_deref()).await?;
            results.extend(response.tasks);

            next_token = response.next_page_token;
            if next_token.is_none() {
                break;
            }

            page += 1;
        }

        Ok(results)
    }

    /// Creates a task within the service.
    ///
    /// This method makes a request to the `POST /tasks` endpoint.
    pub async fn create_task(&self, task: Task) -> Result<CreateTask> {
        self.post("tasks", task).await
    }

    /// Gets a specific task within the service.
    ///
    /// This method makes a request to the `GET /tasks/{id}` endpoint.
    pub async fn get_task(&self, id: impl AsRef<str>, view: View) -> Result<task::Response> {
        let url = format!("tasks/{}?view={view}", id.as_ref());

        Ok(match view {
            View::Minimal => task::Response::Minimal(self.get(url).await?),
            View::Basic => task::Response::Basic(self.get(url).await?),
            View::Full => task::Response::Full(self.get(url).await?),
        })
    }

    /// Cancels a task within the service.
    ///
    /// This method makes a request to the `POST /tasks/{id}:cancel` endpoint.
    pub async fn cancel_task(&self, id: impl AsRef<str>) -> Result<()> {
        self.post(format!("tasks/{}:cancel", id.as_ref()), ()).await
    }
}
