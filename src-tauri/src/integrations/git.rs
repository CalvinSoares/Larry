use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

const MAX_GIT_OUTPUT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshot {
    pub repository_root: String,
    pub branch: String,
    pub status: String,
    pub diff: String,
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

    Ok(GitSnapshot {
        repository_root: display_path(&repository_root),
        branch: if branch.trim().is_empty() {
            "HEAD detached".to_string()
        } else {
            branch.trim().to_string()
        },
        status,
        diff,
    })
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
}
