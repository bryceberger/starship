// Did not want to depend on all of jj-cli. Most of these are inlined from:
// https://github.com/jj-vcs/jj/blob/6c14ccd89df3f4445ba0e362c17cdd56a13127af/cli/src/diff_util.rs
//
// The original code had the following license notification:
//
// Copyright 2020-2022 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io;

use futures::executor::{block_on, block_on_stream};
use jj_lib::{
    commit::Commit,
    conflicts::{
        ConflictMarkerStyle, ConflictMaterializeOptions, MaterializedFileConflictValue,
        MaterializedFileValue, MaterializedTreeDiffEntry, MaterializedTreeValue,
        materialize_merge_result_to_bytes,
    },
    copies::CopyRecords,
    diff::{CompareBytesExactly, ContentDiff, DiffHunkKind, find_line_ranges},
    repo::Repo,
    repo_path::RepoPath,
    tree_merge::MergeOptions,
};
use tokio::io::AsyncReadExt;

use super::OrLog;
use crate::context::JJRepo;

// from show_diff_stat() in jj_cli/src/diff_util.rs
pub fn run_diff(
    mut diff_tree: impl Iterator<Item = MaterializedTreeDiffEntry>,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    diff_tree.try_fold(
        (0, 0),
        |mut sums, MaterializedTreeDiffEntry { path, values }| {
            let (left, right) = values?;
            let left_path = path.source();
            let right_path = path.target();
            let left_content = diff_content(left_path, left)?;
            let right_content = diff_content(right_path, right)?;

            let (added, deleted) = get_diff_stat(&left_content, &right_content);
            sums.0 += added;
            sums.1 += deleted;

            Ok::<_, Box<dyn std::error::Error>>(sums)
        },
    )
}

fn get_diff_stat(left: &[u8], right: &[u8]) -> (usize, usize) {
    let diff = ContentDiff::for_tokenizer([left, right], find_line_ranges, CompareBytesExactly);
    let mut added = 0;
    let mut removed = 0;
    for hunk in diff.hunks() {
        match hunk.kind {
            DiffHunkKind::Matching => {}
            DiffHunkKind::Different => {
                let [left, right] = hunk.contents[..].try_into().unwrap();
                removed += left.split_inclusive(|b| *b == b'\n').count();
                added += right.split_inclusive(|b| *b == b'\n').count();
            }
        }
    }
    (added, removed)
}

fn diff_content(path: &RepoPath, value: MaterializedTreeValue) -> io::Result<Vec<u8>> {
    match value {
        MaterializedTreeValue::Absent => Ok(Vec::new()),
        MaterializedTreeValue::AccessDenied(err) => {
            Ok(format!("Access denied: {err}").into_bytes())
        }
        MaterializedTreeValue::File(MaterializedFileValue { mut reader, .. }) => {
            let mut buf = Vec::new();
            block_on(reader.read_to_end(&mut buf))?;
            Ok(buf)
        }
        MaterializedTreeValue::Symlink { id: _, target } => Ok(target.into_bytes()),
        MaterializedTreeValue::GitSubmodule(id) => {
            Ok(format!("Git submodule checked out at {id}").into_bytes())
        }
        MaterializedTreeValue::FileConflict(MaterializedFileConflictValue { contents, .. }) => {
            let opts = ConflictMaterializeOptions {
                marker_style: ConflictMarkerStyle::Git,
                marker_len: None,
                merge: MergeOptions {
                    hunk_level: jj_lib::files::FileMergeHunkLevel::Line,
                    same_change: jj_lib::merge::SameChange::Accept,
                },
            };
            Ok(materialize_merge_result_to_bytes(&contents, &opts).into())
        }
        MaterializedTreeValue::OtherConflict { id } => Ok(id.describe().into_bytes()),
        MaterializedTreeValue::Tree(id) => {
            panic!("Unexpected tree with id {id:?} in diff at path {path:?}");
        }
    }
}

pub fn get_copy_records(
    mod_name: &str,
    repo: &JJRepo,
    wc: &Commit,
    copy_records: &mut CopyRecords,
    p: &Commit,
) -> Option<()> {
    let records = repo
        .repo
        .store()
        .get_copy_records(None, p.id(), wc.id())
        .or_log(mod_name)?;
    copy_records
        .add_records(block_on_stream(records))
        .or_log(mod_name)
}
