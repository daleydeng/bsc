use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};

use crate::environment::PreparedEnvironment;

const TEST_RUN_DIRECTORIES: &[&str] = &["rust-test-work", "rust-test-artifacts"];

pub fn prune(environment: &PreparedEnvironment) -> Result<()> {
    let removed = prune_test_runs(&environment.root)?;
    if removed.is_empty() {
        println!("No disposable Rust test run directories were present.");
    } else {
        for directory in removed {
            println!("Removed {}", directory.display());
        }
    }
    println!("Cargo target and persistent test/compiler caches were preserved.");
    Ok(())
}

fn prune_test_runs(project_root: &Path) -> Result<Vec<PathBuf>> {
    let temp_root = project_root.join(".pixi/tmp");
    let mut removed = Vec::new();
    for name in TEST_RUN_DIRECTORIES {
        let directory = temp_root.join(name);
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("could not inspect {}", directory.display()))
            }
        };
        ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "refusing to recursively remove non-directory test path: {}",
            directory.display()
        );
        fs::remove_dir_all(&directory)
            .with_context(|| format!("could not remove {}", directory.display()))?;
        removed.push(directory);
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::prune_test_runs;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn prune_removes_only_disposable_test_runs() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::current_dir()
            .unwrap()
            .join(".pixi/tmp/xtask-tests")
            .join(format!("test-prune-{}-{nonce}", std::process::id()));
        for relative in [
            ".pixi/tmp/rust-test-work/run/case",
            ".pixi/tmp/rust-test-artifacts/run/case",
            ".pixi/tmp/cargo-target/debug",
            ".pixi/cache/rust-tests/entry",
        ] {
            fs::create_dir_all(root.join(relative)).unwrap();
        }

        let removed = prune_test_runs(&root).unwrap();
        assert_eq!(removed.len(), 2);
        assert!(!root.join(".pixi/tmp/rust-test-work").exists());
        assert!(!root.join(".pixi/tmp/rust-test-artifacts").exists());
        assert!(root.join(".pixi/tmp/cargo-target/debug").is_dir());
        assert!(root.join(".pixi/cache/rust-tests/entry").is_dir());

        fs::remove_dir_all(root).unwrap();
    }
}
