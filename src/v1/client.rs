//! A client for interacting with a Task Execution Service (TES) service.

use reqwest_middleware::ClientWithMiddleware as ReqwestClient;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::trace;
use url::Url;

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
mod options;

pub use builder::Builder;
pub use options::Options;

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

    /// A middleware error from `reqwest_middleware`.
    // Note: `reqwest_middleware` stores these as an [`anyhow::Error`] internally.
    #[error(transparent)]
    Middleware(#[from] anyhow::Error),

    /// An error from `reqwest`.
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

/// A [`Result`](std::result::Result) with an [`Error`].
type Result<T> = std::result::Result<T, Error>;

impl From<reqwest_middleware::Error> for Error {
    fn from(value: reqwest_middleware::Error) -> Self {
        match value {
            reqwest_middleware::Error::Middleware(err) => Error::Middleware(err),
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

        let bytes = self.client.get(url).send().await?.bytes().await?;

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
    async fn post<Body, Response>(&self, endpoint: impl AsRef<str>, body: Body) -> Result<Response>
    where
        Body: Serialize,
        Response: for<'de> Deserialize<'de>,
    {
        let endpoint = endpoint.as_ref();
        let body = serde_json::to_string(&body)?;

        // SAFETY: as described in the documentation for this method, the URL is
        // already validated upon creation of the [`Client`], and the
        // `endpoint` is assumed to always be joinable to that URL, so this
        // should always unwrap.
        let url = self.url.join(endpoint).unwrap();
        debug!("POST {url} {body}");

        Ok(self
            .client
            .post(url)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .json::<Response>()
            .await?)
    }

    /// Gets the service information.
    ///
    /// This method makes a request to the `GET /service-info` endpoint.
    pub async fn service_info(&self) -> Result<ServiceInfo> {
        self.get("service-info").await
    }

    /// Lists tasks within the service.
    ///
    /// This method makes a request to the `GET /tasks` endpoint.
    pub async fn list_tasks(
        &self,
        params: Option<&ListTasksParams>,
    ) -> Result<ListTasks<TaskResponse>> {
        if let Some(params) = params {
            if params.page_size.unwrap_or(DEFAULT_PAGE_SIZE) >= MAX_PAGE_SIZE {
                return Err(Error::InvalidRequest(format!(
                    "page size must be less than {MAX_PAGE_SIZE}"
                )));
            }
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
                let results = self.get::<ListTasks<MinimalTask>>(url).await?;

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
                let results = self.get::<ListTasks<responses::Task>>(url).await?;

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
                let results = self.get::<ListTasks<responses::Task>>(url).await?;

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
    /// This method makes a request to the `POST /tasks` endpoint.
    pub async fn create_task(&self, task: &requests::Task) -> Result<CreatedTask> {
        self.post("tasks", task).await
    }

    /// Gets a specific task within the service.
    ///
    /// This method makes a request to the `GET /tasks/{id}` endpoint.
    pub async fn get_task(
        &self,
        id: impl AsRef<str>,
        params: Option<&GetTaskParams>,
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
            View::Minimal => TaskResponse::Minimal(self.get(url).await?),
            View::Basic => TaskResponse::Basic(self.get(url).await?),
            View::Full => TaskResponse::Full(self.get(url).await?),
        })
    }

    /// Cancels a task within the service.
    ///
    /// This method makes a request to the `POST /tasks/{id}:cancel` endpoint.
    pub async fn cancel_task(&self, id: impl AsRef<str>) -> Result<()> {
        self.post(format!("tasks/{}:cancel", id.as_ref()), ()).await
    }
}
