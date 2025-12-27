use futures::executor::{block_on, block_on_stream};
use jj_lib::{
    backend::ChangeId,
    commit::Commit,
    conflicts::materialized_diff_stream,
    copies::CopyRecords,
    id_prefix::{IdPrefixContext, IdPrefixIndex},
    index::IndexResult,
    matchers::EverythingMatcher,
    repo::{ReadonlyRepo, Repo},
    rewrite::merge_commit_trees,
};

use super::{Context, Module, ModuleConfig};
use crate::{
    configs::jujutsu::{JujutsuCommitConfig, JujutsuDiffConfig},
    context::JJRepo,
    formatter::StringFormatter,
};

mod util;

pub fn module_commit<'a>(context: &'a Context) -> Option<Module<'a>> {
    let mod_name = "jujutsu_commit";
    let mut module = context.new_module(mod_name);
    let config = JujutsuCommitConfig::try_load(module.config);

    let repo = context.get_repo()?.as_jj()?;
    let wc = get_working_copy(repo, mod_name)?;

    let ctx = IdPrefixContext::new(Default::default());
    let index = ctx.populate(repo.repo.as_ref()).or_log(mod_name)?;

    let (prefix, rest) =
        shortest(repo.repo.as_ref(), &index, wc.change_id(), 8).or_log(mod_name)?;
    let op_id = repo.repo.op_id().to_string();

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
                "operation" => Some(Ok(&op_id)),
                _ => None,
            })
            .parse(None, Some(context))
    });

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `{mod_name}`:\n{error}");
            return None;
        }
    });

    Some(module)
}

pub fn module_diff<'a>(context: &'a Context) -> Option<Module<'a>> {
    let mod_name = "jujutsu_diff";
    let mut module = context.new_module(mod_name);
    let config = JujutsuDiffConfig::try_load(module.config);

    let repo = context.get_repo()?.as_jj()?;
    let wc = get_working_copy(repo, mod_name)?;

    let parents = wc
        .parents()
        .collect::<Result<Vec<_>, _>>()
        .or_log(mod_name)?;
    let from_tree = block_on(merge_commit_trees(repo.repo.as_ref(), &parents)).or_log(mod_name)?;
    let to_tree = wc.tree();

    let mut copy_records = CopyRecords::default();
    for p in &parents {
        util::get_copy_records(mod_name, repo, &wc, &mut copy_records, p)?;
    }

    let diff = from_tree.diff_stream_with_copies(&to_tree, &EverythingMatcher, &copy_records);
    let diff = materialized_diff_stream(repo.repo.store(), diff);

    let (added, deleted) = util::run_diff(block_on_stream(diff)).or_log(mod_name)?;
    let added = if config.only_nonzero_diffs && added == 0 {
        None
    } else {
        Some(added)
    };
    let deleted = if config.only_nonzero_diffs && deleted == 0 {
        None
    } else {
        Some(deleted)
    };

    let parsed = StringFormatter::new(config.format).and_then(|formatter| {
        formatter
            .map_style(|variable| match variable {
                "added_style" => Some(Ok(config.added_style)),
                "deleted_style" => Some(Ok(config.deleted_style)),
                _ => None,
            })
            .map(|variable| match variable {
                "added" => added.map(|v| Ok(format!("{v}"))),
                "deleted" => deleted.map(|v| Ok(format!("{v}"))),
                _ => None,
            })
            .parse(None, Some(context))
    });

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `{mod_name}`:\n{error}");
            return None;
        }
    });

    Some(module)
}

fn get_working_copy(repo: &JJRepo, mod_name: &str) -> Option<Commit> {
    repo.repo
        .store()
        .get_commit(repo.repo.view().get_wc_commit_id(&repo.workspace_name)?)
        .or_log(mod_name)
}

fn shortest(
    repo: &ReadonlyRepo,
    index: &IdPrefixIndex,
    id: &ChangeId,
    total_len: usize,
) -> IndexResult<(String, String)> {
    let prefix_len = index.shortest_change_prefix_len(repo, id)?;
    let mut hex = id.reverse_hex();
    hex.truncate(total_len);
    let rest = hex.split_off(prefix_len);
    Ok((hex, rest))
}

trait OrLog {
    type Output;
    fn or_log(self, module: &str) -> Self::Output;
}

impl<T, E: std::fmt::Display> OrLog for Result<T, E> {
    type Output = Option<T>;

    fn or_log(self, module: &str) -> Self::Output {
        self.inspect_err(|e| log::warn!("in {module}: {e}")).ok()
    }
}
