use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::domain::collection::CollectionFile;
use crate::domain::request::{
    CookieEntry, HeaderEntry, MultipartBody, QueryParam, RequestAuth, RequestBody,
    RequestDefinition,
};

const MAX_GIT_OUTPUT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshot {
    pub repository_root: String,
    pub branch: String,
    pub status: String,
    pub diff: String,
    pub semantic_base: String,
    pub semantic_changes: Vec<GitSemanticChange>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitSemanticChange {
    pub change_type: String,
    pub target: String,
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone)]
struct FlatFolder {
    name: String,
    path: String,
}

#[derive(Debug, Clone)]
struct FlatRequest {
    request: RequestDefinition,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitError {
    pub kind: String,
    pub message: String,
}

impl GitError {
    fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

pub fn inspect_collection(path: &str) -> Result<GitSnapshot, GitError> {
    let collection_path = validate_collection_path(path)?;
    let collection_parent = collection_path.parent().ok_or_else(|| {
        GitError::new(
            "invalid_path",
            "A collection precisa estar dentro de uma pasta.",
        )
    })?;
    let repository_root = find_repository_root(collection_parent)?;
    let relative_collection_path = collection_path
        .strip_prefix(&repository_root)
        .map_err(|error| GitError::new("relative_path", error.to_string()))?;

    let branch = run_git(&repository_root, &["branch", "--show-current"], "branch")?;
    let status = run_git(
        &repository_root,
        &["status", "--short", "--branch", "--untracked-files=normal"],
        "status",
    )?;
    let diff = run_git_for_path(
        &repository_root,
        &["diff", "--no-ext-diff", "--"],
        relative_collection_path,
        "diff",
    )?;

    let current_content = read_collection_content(&collection_path)?;
    let relative_git_path = relative_collection_path
        .to_string_lossy()
        .replace('\\', "/");
    let (semantic_base, semantic_changes) =
        match read_head_file(&repository_root, &relative_git_path)? {
            Some(base_content) => match semantic_diff(&base_content, &current_content) {
                Ok(changes) => ("HEAD".to_string(), changes),
                Err(error) => (format!("Indisponível: {}", error.message), Vec::new()),
            },
            None => (
                "HEAD indisponível para este arquivo".to_string(),
                Vec::new(),
            ),
        };

    Ok(GitSnapshot {
        repository_root: display_path(&repository_root),
        branch: if branch.trim().is_empty() {
            "HEAD detached".to_string()
        } else {
            branch.trim().to_string()
        },
        status,
        diff,
        semantic_base,
        semantic_changes,
    })
}

fn read_collection_content(path: &Path) -> Result<String, GitError> {
    let metadata =
        fs::metadata(path).map_err(|error| GitError::new("metadata", error.to_string()))?;
    if metadata.len() > MAX_GIT_OUTPUT_BYTES as u64 {
        return Err(GitError::new(
            "collection_too_large",
            "A collection excede o limite local de 1 MiB para análise semântica.",
        ));
    }

    fs::read_to_string(path).map_err(|error| GitError::new("read", error.to_string()))
}

fn read_head_file(repository_root: &Path, relative_path: &str) -> Result<Option<String>, GitError> {
    let reference = format!("HEAD:{relative_path}");
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["show", &reference])
        .output()
        .map_err(|error| GitError::new("git_unavailable", error.to_string()))?;

    if !output.status.success() {
        return Ok(None);
    }

    if output.stdout.len() > MAX_GIT_OUTPUT_BYTES {
        return Err(GitError::new(
            "git_output_too_large",
            "A versão HEAD da collection excede o limite local de 1 MiB.",
        ));
    }

    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

fn semantic_diff(
    base_content: &str,
    current_content: &str,
) -> Result<Vec<GitSemanticChange>, GitError> {
    let base = parse_collection(base_content, "HEAD")?;
    let current = parse_collection(current_content, "arquivo atual")?;
    let mut changes = Vec::new();

    if base.name != current.name {
        push_change(
            &mut changes,
            "changed",
            "Collection",
            "Nome",
            Some(base.name.clone()),
            Some(current.name.clone()),
        );
    }

    let mut base_folders = BTreeMap::new();
    let mut current_folders = BTreeMap::new();
    let mut base_requests = BTreeMap::new();
    let mut current_requests = BTreeMap::new();
    flatten_collection(&base, &mut base_folders, &mut base_requests);
    flatten_collection(&current, &mut current_folders, &mut current_requests);

    compare_folders(&base_folders, &current_folders, &mut changes);
    compare_requests(&base_requests, &current_requests, &mut changes);

    Ok(changes)
}

fn parse_collection(content: &str, source: &str) -> Result<CollectionFile, GitError> {
    let mut collection: CollectionFile = serde_yaml::from_str(content).map_err(|error| {
        GitError::new(
            "semantic_parse",
            format!("Não foi possível interpretar {source}: {error}"),
        )
    })?;

    if collection.schema_version == 1 {
        collection.schema_version = crate::domain::collection::CURRENT_COLLECTION_SCHEMA_VERSION;
    }

    collection.validate().map_err(|error| {
        GitError::new(
            "semantic_validation",
            format!("A collection {source} falhou na validação: {error:?}"),
        )
    })?;

    Ok(collection)
}

fn flatten_collection(
    collection: &CollectionFile,
    folders: &mut BTreeMap<String, FlatFolder>,
    requests: &mut BTreeMap<String, FlatRequest>,
) {
    for request in &collection.requests {
        requests.insert(
            request.id.clone(),
            FlatRequest {
                request: request.clone(),
                path: request.name.clone(),
            },
        );
    }

    flatten_folders(&collection.folders, "", folders, requests);
}

fn flatten_folders(
    folders: &[crate::domain::collection::CollectionFolder],
    parent_path: &str,
    flat_folders: &mut BTreeMap<String, FlatFolder>,
    requests: &mut BTreeMap<String, FlatRequest>,
) {
    for folder in folders {
        let path = if parent_path.is_empty() {
            folder.name.clone()
        } else {
            format!("{parent_path} / {}", folder.name)
        };

        flat_folders.insert(
            folder.id.clone(),
            FlatFolder {
                name: folder.name.clone(),
                path: path.clone(),
            },
        );

        for request in &folder.requests {
            requests.insert(
                request.id.clone(),
                FlatRequest {
                    request: request.clone(),
                    path: format!("{path} / {}", request.name),
                },
            );
        }

        flatten_folders(&folder.folders, &path, flat_folders, requests);
    }
}

fn compare_folders(
    base: &BTreeMap<String, FlatFolder>,
    current: &BTreeMap<String, FlatFolder>,
    changes: &mut Vec<GitSemanticChange>,
) {
    let ids = base
        .keys()
        .chain(current.keys())
        .collect::<std::collections::BTreeSet<_>>();

    for id in ids {
        match (base.get(id), current.get(id)) {
            (None, Some(folder)) => push_change(
                changes,
                "added",
                &folder.path,
                "Pasta",
                None,
                Some("adicionada".to_string()),
            ),
            (Some(folder), None) => push_change(
                changes,
                "removed",
                &folder.path,
                "Pasta",
                Some("presente".to_string()),
                None,
            ),
            (Some(before), Some(after)) => {
                if before.name != after.name {
                    push_change(
                        changes,
                        "changed",
                        &after.path,
                        "Nome",
                        Some(before.name.clone()),
                        Some(after.name.clone()),
                    );
                }

                if parent_path(&before.path) != parent_path(&after.path) {
                    push_change(
                        changes,
                        "changed",
                        &after.path,
                        "Localização",
                        Some(parent_path(&before.path)),
                        Some(parent_path(&after.path)),
                    );
                }
            }
            (None, None) => {}
        }
    }
}

fn compare_requests(
    base: &BTreeMap<String, FlatRequest>,
    current: &BTreeMap<String, FlatRequest>,
    changes: &mut Vec<GitSemanticChange>,
) {
    let ids = base
        .keys()
        .chain(current.keys())
        .collect::<std::collections::BTreeSet<_>>();

    for id in ids {
        match (base.get(id), current.get(id)) {
            (None, Some(request)) => push_change(
                changes,
                "added",
                &request.path,
                "Request",
                None,
                Some(request_summary(&request.request)),
            ),
            (Some(request), None) => push_change(
                changes,
                "removed",
                &request.path,
                "Request",
                Some(request_summary(&request.request)),
                None,
            ),
            (Some(before), Some(after)) => compare_request(before, after, changes),
            (None, None) => {}
        }
    }
}

fn compare_request(
    before: &FlatRequest,
    after: &FlatRequest,
    changes: &mut Vec<GitSemanticChange>,
) {
    let target = &after.path;
    if before.request.name != after.request.name {
        push_change(
            changes,
            "changed",
            target,
            "Nome",
            Some(before.request.name.clone()),
            Some(after.request.name.clone()),
        );
    }

    if parent_path(&before.path) != parent_path(&after.path) {
        push_change(
            changes,
            "changed",
            target,
            "Localização",
            Some(parent_path(&before.path)),
            Some(parent_path(&after.path)),
        );
    }

    let before_method = method_label(&before.request);
    let after_method = method_label(&after.request);
    if before_method != after_method {
        push_change(
            changes,
            "changed",
            target,
            "Método",
            Some(before_method),
            Some(after_method),
        );
    }

    if before.request.url != after.request.url {
        push_change(
            changes,
            "changed",
            target,
            "URL",
            Some(redact_url(&before.request.url)),
            Some(redact_url(&after.request.url)),
        );
    }

    compare_entries(
        changes,
        target,
        "Query params",
        query_summary(&before.request.query),
        query_summary(&after.request.query),
        &before.request.query,
        &after.request.query,
    );
    compare_entries(
        changes,
        target,
        "Headers",
        header_summary(&before.request.headers),
        header_summary(&after.request.headers),
        &before.request.headers,
        &after.request.headers,
    );
    compare_entries(
        changes,
        target,
        "Cookies",
        cookie_summary(&before.request.cookies),
        cookie_summary(&after.request.cookies),
        &before.request.cookies,
        &after.request.cookies,
    );

    if before.request.body != after.request.body {
        push_change(
            changes,
            "changed",
            target,
            "Body",
            Some(body_summary(&before.request.body)),
            Some(body_summary(&after.request.body)),
        );
    }

    if before.request.auth != after.request.auth {
        push_change(
            changes,
            "changed",
            target,
            "Auth",
            Some(auth_summary(&before.request.auth)),
            Some(auth_summary(&after.request.auth)),
        );
    }

    if before.request.assertions != after.request.assertions {
        push_change(
            changes,
            "changed",
            target,
            "Assertions",
            Some(format!("{} regra(s)", before.request.assertions.len())),
            Some(format!("{} regra(s)", after.request.assertions.len())),
        );
    }
}

fn compare_entries<T: PartialEq>(
    changes: &mut Vec<GitSemanticChange>,
    target: &str,
    field: &str,
    before_summary: String,
    after_summary: String,
    before: &[T],
    after: &[T],
) {
    if before != after {
        push_change(
            changes,
            "changed",
            target,
            field,
            Some(before_summary),
            Some(after_summary),
        );
    }
}

fn push_change(
    changes: &mut Vec<GitSemanticChange>,
    change_type: &str,
    target: &str,
    field: &str,
    before: Option<String>,
    after: Option<String>,
) {
    changes.push(GitSemanticChange {
        change_type: change_type.to_string(),
        target: target.to_string(),
        field: field.to_string(),
        before,
        after,
    });
}

fn parent_path(path: &str) -> String {
    path.rsplit_once(" / ")
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_else(|| "Raiz".to_string())
}

fn request_summary(request: &RequestDefinition) -> String {
    format!("{} {}", method_label(request), redact_url(&request.url))
}

fn redact_url(value: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(value) else {
        return "[URL não interpretável]".to_string();
    };

    let pairs = url
        .query_pairs()
        .map(|(name, value)| {
            if is_sensitive_key(&name) {
                (name.to_string(), "[redacted]".to_string())
            } else {
                (name.to_string(), value.to_string())
            }
        })
        .collect::<Vec<_>>();

    if pairs.is_empty() {
        return url.to_string();
    }

    {
        let mut query = url.query_pairs_mut();
        query.clear();
        for (name, value) in &pairs {
            query.append_pair(name, value);
        }
    }

    url.to_string()
}

fn is_sensitive_key(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "authorization",
        "access_token",
        "api_key",
        "apikey",
        "client_secret",
        "credential",
        "cookie",
        "password",
        "secret",
        "token",
    ]
    .iter()
    .any(|part| value.contains(part))
}

fn method_label(request: &RequestDefinition) -> String {
    serde_json::to_value(&request.method)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "HTTP".to_string())
}

fn query_summary(entries: &[QueryParam]) -> String {
    named_entries_summary(entries.iter().map(|entry| entry.name.as_str()))
}

fn header_summary(entries: &[HeaderEntry]) -> String {
    named_entries_summary(entries.iter().map(|entry| entry.name.as_str()))
}

fn cookie_summary(entries: &[CookieEntry]) -> String {
    named_entries_summary(entries.iter().map(|entry| entry.name.as_str()))
}

fn named_entries_summary<'a>(entries: impl Iterator<Item = &'a str>) -> String {
    let names = entries
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if names.is_empty() {
        "0 entradas".to_string()
    } else {
        format!("{}: {}", names.len(), names.join(", "))
    }
}

fn body_summary(body: &Option<RequestBody>) -> String {
    match body {
        None => "Sem body".to_string(),
        Some(RequestBody::Json(_)) => "JSON".to_string(),
        Some(RequestBody::Text(_)) => "Texto".to_string(),
        Some(RequestBody::FormUrlEncoded(fields)) => {
            format!("Form URL Encoded ({} campos)", fields.len())
        }
        Some(RequestBody::Multipart(MultipartBody { fields, files })) => {
            format!(
                "Multipart ({} campos, {} arquivos)",
                fields.len(),
                files.len()
            )
        }
    }
}

fn auth_summary(auth: &Option<RequestAuth>) -> String {
    match auth {
        None => "Sem autenticação".to_string(),
        Some(RequestAuth::Bearer { .. }) => "Bearer token".to_string(),
        Some(RequestAuth::Basic { .. }) => "Basic Auth".to_string(),
        Some(RequestAuth::ApiKey { location, .. }) => {
            format!("API key ({location:?})")
        }
    }
}

fn validate_collection_path(path: &str) -> Result<PathBuf, GitError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(GitError::new(
            "invalid_path",
            "Informe o caminho da collection.",
        ));
    }

    let path = PathBuf::from(trimmed);
    if path.extension().and_then(|extension| extension.to_str()) != Some("yaml") {
        return Err(GitError::new(
            "invalid_path",
            "O caminho precisa apontar para uma collection .yaml.",
        ));
    }

    let metadata =
        fs::metadata(&path).map_err(|error| GitError::new("metadata", error.to_string()))?;
    if !metadata.is_file() {
        return Err(GitError::new(
            "invalid_path",
            "O caminho da collection precisa apontar para um arquivo.",
        ));
    }

    fs::canonicalize(path).map_err(|error| GitError::new("canonicalize", error.to_string()))
}

fn find_repository_root(start_directory: &Path) -> Result<PathBuf, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(start_directory)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| GitError::new("git_unavailable", error.to_string()))?;

    if !output.status.success() {
        return Err(GitError::new(
            "not_git_repository",
            git_failure_message("rev-parse", &output),
        ));
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        return Err(GitError::new(
            "invalid_repository_root",
            "O Git não retornou a raiz do repositório.",
        ));
    }

    fs::canonicalize(root).map_err(|error| GitError::new("canonicalize", error.to_string()))
}

fn run_git(repository_root: &Path, args: &[&str], operation: &str) -> Result<String, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(args)
        .output()
        .map_err(|error| GitError::new("git_unavailable", error.to_string()))?;

    read_successful_output(operation, output)
}

fn run_git_for_path(
    repository_root: &Path,
    args: &[&str],
    path: &Path,
    operation: &str,
) -> Result<String, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(args)
        .arg(path)
        .output()
        .map_err(|error| GitError::new("git_unavailable", error.to_string()))?;

    read_successful_output(operation, output)
}

fn read_successful_output(
    operation: &str,
    output: std::process::Output,
) -> Result<String, GitError> {
    if output.stdout.len() > MAX_GIT_OUTPUT_BYTES {
        return Err(GitError::new(
            "git_output_too_large",
            format!("A saída de {operation} excede o limite local de 1 MiB."),
        ));
    }

    if !output.status.success() {
        return Err(GitError::new(
            "git_command_failed",
            git_failure_message(operation, &output),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_failure_message(operation: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("A operação Git {operation} falhou.")
    } else {
        format!("A operação Git {operation} falhou: {stderr}")
    }
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use super::*;

    #[test]
    fn inspeciona_repositorio_git_local() {
        let root = std::env::temp_dir().join(format!("larry-git-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        Command::new("git")
            .args(["-C"])
            .arg(&root)
            .args(["init", "--quiet"])
            .status()
            .unwrap();

        let collection = root.join("collection.yaml");
        fs::write(&collection, "schemaVersion: 1\nname: Test\nrequests: []\n").unwrap();

        let snapshot = inspect_collection(&collection.to_string_lossy()).unwrap();

        assert_eq!(
            snapshot.repository_root,
            display_path(&fs::canonicalize(&root).unwrap())
        );
        assert!(snapshot.status.contains("##"));
        assert!(snapshot.status.contains("collection.yaml"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejeita_caminho_sem_extensao_yaml() {
        let error = validate_collection_path("collection.json").unwrap_err();

        assert_eq!(error.kind, "invalid_path");
    }

    #[test]
    fn diff_semantico_identifica_campos_sem_expor_secret() {
        let base = r#"
schemaVersion: 2
name: Payments
requests:
  - id: payment
    name: Create payment
    method: POST
    url: https://api.example.com/payments
    query: []
    headers:
      - name: Authorization
        value: Bearer old-secret
        enabled: true
    body:
      type: json
      value:
        amount: 100
folders: []
"#;
        let current = base
            .replace(
                "https://api.example.com/payments",
                "https://staging.example.com/payments",
            )
            .replace("Bearer old-secret", "Bearer new-secret")
            .replace("amount: 100", "amount: 200");

        let changes = semantic_diff(base, &current).unwrap();
        let serialized = serde_json::to_string(&changes).unwrap();

        assert!(changes.iter().any(|change| change.field == "URL"));
        assert!(changes.iter().any(|change| change.field == "Headers"));
        assert!(changes.iter().any(|change| change.field == "Body"));
        assert!(!serialized.contains("old-secret"));
        assert!(!serialized.contains("new-secret"));
    }

    #[test]
    fn redige_query_sensivel_no_diff() {
        let value = redact_url("https://api.example.com/payments?token=secret-value&region=br");

        assert!(value.contains("token=%5Bredacted%5D"));
        assert!(value.contains("region=br"));
        assert!(!value.contains("secret-value"));
    }
}
