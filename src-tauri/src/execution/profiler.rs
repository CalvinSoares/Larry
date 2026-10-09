use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, watch, Semaphore};

use crate::domain::environment::EnvironmentFile;
use crate::domain::request::RequestDefinition;
use crate::execution::http::{execute_with_redactions, ExecutionError};
use crate::execution::variables::resolve_request;

const MAX_PROFILER_REQUESTS: u32 = 1_000;
const MAX_PROFILER_CONCURRENCY: u32 = 32;
static PROFILER_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilerConfig {
    pub total_requests: u32,
    pub concurrency: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilerStart {
    pub run_id: String,
    pub total_requests: u32,
    pub concurrency: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilerStatusCount {
    pub status: u16,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilerProvenance {
    pub latency: String,
    pub throughput: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilerSummary {
    pub run_id: String,
    pub requested: u32,
    pub completed: u32,
    pub cancelled_requests: u32,
    pub successful: u32,
    pub http_errors: u32,
    pub transport_errors: u32,
    pub elapsed_ms: u64,
    pub throughput_rps: f64,
    pub average_ms: f64,
    pub min_ms: Option<u64>,
    pub max_ms: Option<u64>,
    pub p50_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub p99_ms: Option<u64>,
    pub statuses: Vec<ProfilerStatusCount>,
    pub provenance: ProfilerProvenance,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilerError {
    pub kind: String,
    pub message: String,
    pub technical: Option<String>,
}

impl ProfilerError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            technical: None,
        }
    }

    fn from_execution(error: ExecutionError) -> Self {
        Self {
            kind: error.kind,
            message: error.message,
            technical: error.diagnostic.map(|diagnostic| diagnostic.technical),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "eventType", rename_all = "camelCase")]
pub enum ProfilerEvent {
    #[serde(rename = "started")]
    Started {
        run_id: String,
        requested: u32,
        concurrency: u32,
    },
    #[serde(rename = "progress")]
    Progress {
        run_id: String,
        completed: u32,
        requested: u32,
        successful: u32,
        http_errors: u32,
        transport_errors: u32,
        cancelled_requests: u32,
        last_latency_ms: Option<u64>,
        last_error: Option<ProfilerError>,
    },
    #[serde(rename = "completed")]
    Completed { summary: ProfilerSummary },
    #[serde(rename = "cancelled")]
    Cancelled { summary: ProfilerSummary },
}

#[derive(Clone, Default)]
pub struct ProfilerManager {
    runs: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>,
}

impl ProfilerManager {
    fn insert(&self, run_id: String, sender: watch::Sender<bool>) {
        if let Ok(mut runs) = self.runs.lock() {
            runs.insert(run_id, sender);
        }
    }

    fn sender(&self, run_id: &str) -> Option<watch::Sender<bool>> {
        self.runs
            .lock()
            .ok()
            .and_then(|runs| runs.get(run_id).cloned())
    }

    fn remove(&self, run_id: &str) {
        if let Ok(mut runs) = self.runs.lock() {
            runs.remove(run_id);
        }
    }
}

#[derive(Debug)]
enum WorkerResult {
    Completed {
        latency_ms: u64,
        status: u16,
    },
    Failed {
        latency_ms: u64,
        error: ProfilerError,
    },
    Cancelled,
}

#[tauri::command]
pub async fn start_profiler(
    app: AppHandle,
    manager: State<'_, ProfilerManager>,
    request: RequestDefinition,
    environment: Option<EnvironmentFile>,
    config: ProfilerConfig,
) -> Result<ProfilerStart, ProfilerError> {
    validate_config(&config)?;

    let resolved =
        resolve_request(request, environment.as_ref()).map_err(|error| ProfilerError {
            kind: error.kind,
            message: error.message,
            technical: None,
        })?;

    let run_id = format!(
        "profiler-{}-{}",
        now_ms(),
        PROFILER_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let (cancel_sender, cancel_receiver) = watch::channel(false);
    manager.insert(run_id.clone(), cancel_sender);
    let total_requests = config.total_requests;
    let concurrency = config.concurrency;

    emit_event(
        &app,
        ProfilerEvent::Started {
            run_id: run_id.clone(),
            requested: config.total_requests,
            concurrency: config.concurrency,
        },
    );

    let manager = manager.inner().clone();
    let run_id_for_task = run_id.clone();
    let app_for_task = app.clone();
    tokio::spawn(async move {
        run_profiler(
            app_for_task,
            manager,
            run_id_for_task,
            resolved.request,
            resolved.redactions,
            config,
            cancel_receiver,
        )
        .await;
    });

    Ok(ProfilerStart {
        run_id,
        total_requests,
        concurrency,
    })
}

#[tauri::command]
pub async fn cancel_profiler(
    manager: State<'_, ProfilerManager>,
    run_id: String,
) -> Result<(), ProfilerError> {
    let sender = manager.sender(&run_id).ok_or_else(|| {
        ProfilerError::new(
            "profiler_not_found",
            "A execução do profiler não está ativa.",
        )
    })?;

    sender.send(true).map_err(|_| {
        ProfilerError::new(
            "profiler_cancel",
            "Não foi possível sinalizar o cancelamento do profiler.",
        )
    })
}

async fn run_profiler(
    app: AppHandle,
    manager: ProfilerManager,
    run_id: String,
    request: RequestDefinition,
    redactions: Vec<String>,
    config: ProfilerConfig,
    cancel_receiver: watch::Receiver<bool>,
) {
    let started_at = Instant::now();
    let semaphore = Arc::new(Semaphore::new(config.concurrency as usize));
    let (result_sender, mut result_receiver) = mpsc::channel(config.total_requests as usize);

    for _ in 0..config.total_requests {
        let semaphore = semaphore.clone();
        let result_sender = result_sender.clone();
        let request = request.clone();
        let redactions = redactions.clone();
        let cancel_receiver = cancel_receiver.clone();

        tokio::spawn(async move {
            if is_cancelled(&cancel_receiver) {
                let _ = result_sender.send(WorkerResult::Cancelled).await;
                return;
            }

            let permit = tokio::select! {
                permit = semaphore.acquire_owned() => permit,
                _ = wait_for_cancellation(cancel_receiver.clone()) => {
                    let _ = result_sender.send(WorkerResult::Cancelled).await;
                    return;
                }
            };

            let Ok(_permit) = permit else {
                let _ = result_sender
                    .send(WorkerResult::Failed {
                        latency_ms: 0,
                        error: ProfilerError::new(
                            "profiler_semaphore",
                            "O profiler não conseguiu reservar uma vaga de concorrência.",
                        ),
                    })
                    .await;
                return;
            };

            if is_cancelled(&cancel_receiver) {
                let _ = result_sender.send(WorkerResult::Cancelled).await;
                return;
            }

            let request_started = Instant::now();
            let result = tokio::select! {
                result = execute_with_redactions(request, &redactions) => result,
                _ = wait_for_cancellation(cancel_receiver) => {
                    let _ = result_sender.send(WorkerResult::Cancelled).await;
                    return;
                }
            };
            let latency_ms = request_started.elapsed().as_millis() as u64;

            let worker_result = match result {
                Ok(response) => WorkerResult::Completed {
                    latency_ms,
                    status: response.status,
                },
                Err(error) => WorkerResult::Failed {
                    latency_ms,
                    error: ProfilerError::from_execution(error),
                },
            };

            let _ = result_sender.send(worker_result).await;
        });
    }
    drop(result_sender);

    let mut latencies = Vec::with_capacity(config.total_requests as usize);
    let mut statuses = BTreeMap::<u16, u32>::new();
    let mut completed = 0;
    let mut cancelled_requests = 0;
    let mut successful = 0;
    let mut http_errors = 0;
    let mut transport_errors = 0;

    for _ in 0..config.total_requests {
        let Some(result) = result_receiver.recv().await else {
            break;
        };

        let mut last_latency_ms = None;
        let mut last_error = None;

        match result {
            WorkerResult::Completed { latency_ms, status } => {
                completed += 1;
                successful += u32::from(status < 400);
                http_errors += u32::from(status >= 400);
                latencies.push(latency_ms);
                *statuses.entry(status).or_default() += 1;
                last_latency_ms = Some(latency_ms);
            }
            WorkerResult::Failed { latency_ms, error } => {
                completed += 1;
                transport_errors += 1;
                latencies.push(latency_ms);
                last_latency_ms = Some(latency_ms);
                last_error = Some(error);
            }
            WorkerResult::Cancelled => {
                cancelled_requests += 1;
            }
        }

        emit_event(
            &app,
            ProfilerEvent::Progress {
                run_id: run_id.clone(),
                completed,
                requested: config.total_requests,
                successful,
                http_errors,
                transport_errors,
                cancelled_requests,
                last_latency_ms,
                last_error,
            },
        );
    }

    latencies.sort_unstable();
    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    let elapsed_seconds = elapsed_ms as f64 / 1_000.0;
    let summary = ProfilerSummary {
        run_id: run_id.clone(),
        requested: config.total_requests,
        completed,
        cancelled_requests,
        successful,
        http_errors,
        transport_errors,
        elapsed_ms,
        throughput_rps: if elapsed_seconds > 0.0 {
            completed as f64 / elapsed_seconds
        } else {
            0.0
        },
        average_ms: average(&latencies),
        min_ms: latencies.first().copied(),
        max_ms: latencies.last().copied(),
        p50_ms: percentile(&latencies, 0.50),
        p95_ms: percentile(&latencies, 0.95),
        p99_ms: percentile(&latencies, 0.99),
        statuses: statuses
            .into_iter()
            .map(|(status, count)| ProfilerStatusCount { status, count })
            .collect(),
        provenance: ProfilerProvenance {
            latency: "measured".to_string(),
            throughput: "inferred".to_string(),
            status: "observed".to_string(),
        },
    };

    if is_cancelled(&cancel_receiver) {
        emit_event(&app, ProfilerEvent::Cancelled { summary });
    } else {
        emit_event(&app, ProfilerEvent::Completed { summary });
    }

    manager.remove(&run_id);
}

async fn wait_for_cancellation(mut receiver: watch::Receiver<bool>) {
    while !*receiver.borrow() {
        if receiver.changed().await.is_err() {
            return;
        }
    }
}

fn is_cancelled(receiver: &watch::Receiver<bool>) -> bool {
    *receiver.borrow()
}

fn emit_event(app: &AppHandle, event: ProfilerEvent) {
    let _ = app.emit("profiler_event", event);
}

fn validate_config(config: &ProfilerConfig) -> Result<(), ProfilerError> {
    if !(1..=MAX_PROFILER_REQUESTS).contains(&config.total_requests) {
        return Err(ProfilerError::new(
            "invalid_profiler_requests",
            "O profiler aceita entre 1 e 1000 requests por execução.",
        ));
    }

    if !(1..=MAX_PROFILER_CONCURRENCY).contains(&config.concurrency) {
        return Err(ProfilerError::new(
            "invalid_profiler_concurrency",
            "A concorrência deve ficar entre 1 e 32 requests simultâneas.",
        ));
    }

    if config.concurrency > config.total_requests {
        return Err(ProfilerError::new(
            "invalid_profiler_concurrency",
            "A concorrência não pode ser maior que a quantidade total de requests.",
        ));
    }

    Ok(())
}

fn average(values: &[u64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.iter().sum::<u64>() as f64 / values.len() as f64
}

fn percentile(values: &[u64], percentile: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }

    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values.get(index).copied()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{average, percentile, validate_config, wait_for_cancellation, ProfilerConfig};

    #[test]
    fn accepts_safe_profiler_configuration() {
        assert!(validate_config(&ProfilerConfig {
            total_requests: 100,
            concurrency: 8,
        })
        .is_ok());
    }

    #[test]
    fn rejects_profiler_limits() {
        for config in [
            ProfilerConfig {
                total_requests: 0,
                concurrency: 1,
            },
            ProfilerConfig {
                total_requests: 1_001,
                concurrency: 1,
            },
            ProfilerConfig {
                total_requests: 10,
                concurrency: 0,
            },
            ProfilerConfig {
                total_requests: 10,
                concurrency: 33,
            },
            ProfilerConfig {
                total_requests: 2,
                concurrency: 3,
            },
        ] {
            assert!(validate_config(&config).is_err());
        }
    }

    #[test]
    fn calculates_latency_statistics() {
        let values = [10, 20, 30, 40, 50];

        assert_eq!(average(&values), 30.0);
        assert_eq!(percentile(&values, 0.50), Some(30));
        assert_eq!(percentile(&values, 0.95), Some(50));
        assert_eq!(percentile(&[], 0.99), None);
    }

    #[tokio::test]
    async fn observes_cancellation_signal() {
        let (sender, receiver) = tokio::sync::watch::channel(false);
        let waiting = tokio::spawn(wait_for_cancellation(receiver));

        sender.send(true).expect("o receiver deve continuar ativo");
        tokio::time::timeout(Duration::from_millis(100), waiting)
            .await
            .expect("o cancelamento deve ser observado")
            .expect("a tarefa de espera deve terminar");
    }
}
