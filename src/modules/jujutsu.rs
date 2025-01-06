use jj_lib::backend::ChangeId;
use jj_lib::id_prefix::{IdPrefixContext, IdPrefixIndex};
use jj_lib::repo::Repo;
use jj_lib::{commit::Commit, repo::ReadonlyRepo};

use crate::formatter::StringFormatter;
use crate::{configs::jujutsu::JujutsuCommitConfig, context::JJRepo};

use super::{Context, Module, ModuleConfig};

pub fn module_commit<'a>(context: &'a Context) -> Option<Module<'a>> {
    let mut module = context.new_module("jujutsu_commit");
    let config = JujutsuCommitConfig::try_load(module.config);

    let repo = context.get_repo()?.as_jj()?;
    let wc = get_working_copy(repo)?;

    let ctx = IdPrefixContext::new(Default::default());
    let index = ctx.populate(repo.repo.as_ref()).or_log()?;

    let (prefix, rest) = shortest(repo.repo.as_ref(), &index, wc.change_id(), 8);

    let desc = wc.description().lines().next();
    let (desc, desc_style) = desc.filter(|d| !d.trim().is_empty()).map_or(
        (config.description_empty, config.style_description_empty),
        |d| (d.trim(), config.style_description),
    );

    let parsed = StringFormatter::new(config.format).and_then(|formatter| {
        formatter
            .map_style(|variable| match variable {
                "style_prefix" => Some(Ok(config.style_prefix)),
                "style_rest" => Some(Ok(config.style_rest)),
                "style_description" => Some(Ok(desc_style)),
                _ => None,
            })
            .map(|variable| match variable {
                "prefix" => Some(Ok(prefix.as_str())),
                "rest" => Some(Ok(rest.as_str())),
                "description" => Some(Ok(desc)),
                _ => None,
            })
            .parse(None, Some(context))
    });

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `jujutsu_commit`:\n{}", error);
            return None;
        }
    });

    Some(module)
}

fn get_working_copy(repo: &JJRepo) -> Option<Commit> {
    repo.repo
        .store()
        .get_commit(repo.repo.view().get_wc_commit_id(&repo.workspace_id)?)
        .or_log()
}

fn shortest(
    repo: &ReadonlyRepo,
    index: &IdPrefixIndex,
    id: &ChangeId,
    total_len: usize,
) -> (String, String) {
    let prefix_len = index.shortest_change_prefix_len(repo, id);
    let mut hex = id.reverse_hex();
    hex.truncate(total_len);
    let rest = hex.split_off(prefix_len);
    (hex, rest)
}

trait OrLog {
    type Output;
    fn or_log(self) -> Self::Output;
}

impl<T, E: std::fmt::Display> OrLog for Result<T, E> {
    type Output = Option<T>;

    fn or_log(self) -> Self::Output {
        self.inspect_err(|e| log::warn!("while getting jj commit info: {e}"))
            .ok()
    }
}
