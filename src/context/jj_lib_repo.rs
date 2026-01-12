use std::path::{Path, PathBuf};
use std::sync::Arc;

use jj_lib::config::StackedConfig;
use jj_lib::default_backend_factories::default_backend_factories;
use jj_lib::default_backend_factories::default_working_copy_factories;
use jj_lib::ref_name::WorkspaceNameBuf;
use jj_lib::repo::ReadonlyRepo;
use jj_lib::settings::UserSettings;
use jj_lib::workspace::Workspace;

use pollster::FutureExt as _;

pub struct Repo {
    pub workdir: PathBuf,
    pub workspace_name: WorkspaceNameBuf,
    pub repo: Arc<ReadonlyRepo>,
}

pub fn init_repo(cwd: &Path) -> Option<Repo> {
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> Option<T> {
        r.inspect_err(|e| log::warn!("while loading jj repo: {e}"))
            .ok()
    }

    let workspace_dir = cwd.ancestors().find(|path| path.join(".jj").is_dir())?;

    let settings = ok(UserSettings::from_config(StackedConfig::with_defaults()))?;
    let store_factories = default_backend_factories();
    let working_copy_factories = default_working_copy_factories();
    let workspace = ok(Workspace::load(
        &settings,
        workspace_dir,
        &store_factories,
        &working_copy_factories,
    ))?;
    let repo = ok(workspace.repo_loader().load_at_head().block_on())?;

    Some(Repo {
        workdir: workspace_dir.into(),
        repo,
        workspace_name: workspace.workspace_name().into(),
    })
}
