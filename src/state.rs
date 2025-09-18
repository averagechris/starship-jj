use std::sync::Arc;

use jj_cli::{
    cli_util::CommandHelper,
    command_error::CommandError,
    diff_util::{DiffStatOptions, DiffStats, get_copy_records},
    ui::Ui,
};
use jj_lib::{
    backend::CommitId,
    commit::Commit,
    copies::CopyRecords,
    fileset::FilesetExpression,
    merged_tree::MergedTree,
    repo::{ReadonlyRepo, Repo},
    workspace::Workspace,
};
use pollster::FutureExt;

type Result<T> = std::result::Result<T, CommandError>;

#[derive(Default)]
pub struct State {
    workspace: Option<Workspace>,
    repo: Option<Arc<ReadonlyRepo>>,
    commit_id: Option<Option<CommitId>>,
    commit: Option<Option<Commit>>,
    tree: Option<Option<MergedTree>>,
    parent_tree: Option<Option<MergedTree>>,
}

impl State {
    pub fn workspace(&mut self, command_helper: &CommandHelper) -> Result<&Workspace> {
        if self.workspace.is_none() {
            let workspace = command_helper.load_workspace()?;
            self.workspace = Some(workspace);
        }
        let Some(w) = self.workspace.as_ref() else {
            unreachable!()
        };
        Ok(w)
    }

    pub fn load_repo(&mut self, command_helper: &CommandHelper) -> Result<()> {
        if self.repo.is_some() {
            return Ok(());
        }
        let repo_loader = self.workspace(command_helper)?.repo_loader();
        let op_head = command_helper.resolve_operation(&Ui::null(), repo_loader)?;
        let repo = repo_loader.load_at(&op_head)?;
        self.repo = Some(repo);
        Ok(())
    }

    pub fn repo(&mut self, command_helper: &CommandHelper) -> Result<Arc<ReadonlyRepo>> {
        self.load_repo(command_helper)?;
        let Some(repo) = &self.repo else {
            unreachable!();
        };
        Ok(repo.clone())
    }

    pub fn load_commit_id(&mut self, command_helper: &CommandHelper) -> Result<()> {
        if self.commit_id.is_some() {
            return Ok(());
        }
        let commit_id = self
            .repo(command_helper)?
            .view()
            .get_wc_commit_id(self.workspace(command_helper)?.workspace_name())
            .cloned();

        self.commit_id = Some(commit_id);
        Ok(())
    }

    pub fn commit_id(&mut self, command_helper: &CommandHelper) -> Result<&Option<CommitId>> {
        self.load_commit_id(command_helper)?;
        let Some(w) = self.commit_id.as_ref() else {
            unreachable!()
        };
        Ok(w)
    }

    pub fn load_commit(&mut self, command_helper: &CommandHelper) -> Result<()> {
        if self.commit.is_some() {
            return Ok(());
        }
        let repo = self.repo(command_helper)?;
        let store = repo.store();
        let commit = self
            .commit_id(command_helper)?
            .as_ref()
            .map(|id| store.get_commit(id))
            .transpose()?;

        self.commit = Some(commit);
        Ok(())
    }
    pub fn commit(&mut self, command_helper: &CommandHelper) -> Result<&Option<Commit>> {
        self.load_commit(command_helper)?;
        let Some(w) = self.commit.as_ref() else {
            unreachable!()
        };
        Ok(w)
    }

    pub fn load_parent_tree(&mut self, command_helper: &CommandHelper) -> Result<()> {
        if self.parent_tree.is_some() {
            return Ok(());
        }
        let repo = self.repo(command_helper)?;
        let commit = self.commit(command_helper)?;
        let parent_tree = commit
            .as_ref()
            .map(|c| c.parent_tree(repo.as_ref()))
            .transpose()?;
        self.parent_tree = Some(parent_tree);
        Ok(())
    }
    pub fn parent_tree(&mut self, command_helper: &CommandHelper) -> Result<&Option<MergedTree>> {
        self.load_parent_tree(command_helper)?;
        let Some(w) = self.parent_tree.as_ref() else {
            unreachable!()
        };
        Ok(w)
    }

    pub fn load_tree(&mut self, command_helper: &CommandHelper) -> Result<()> {
        if self.tree.is_some() {
            return Ok(());
        }
        let commit = self.commit(command_helper)?;
        let tree = commit.as_ref().map(|c| c.tree()).transpose()?;
        self.tree = Some(tree);
        Ok(())
    }

    pub fn tree(&mut self, command_helper: &CommandHelper) -> Result<&Option<MergedTree>> {
        self.load_tree(command_helper)?;
        let Some(w) = self.tree.as_ref() else {
            unreachable!()
        };
        Ok(w)
    }

    pub fn diff_stats(&mut self, command_helper: &CommandHelper) -> Result<Option<DiffStats>> {
        self.load_parent_tree(command_helper)?;
        self.load_tree(command_helper)?;

        let repo = self.repo(command_helper)?;

        let Some(Some(commit)) = self.commit.as_ref() else {
            return Ok(None);
        };
        let store = repo.store();

        let Some(Some(tree)) = self.tree.as_ref() else {
            return Ok(None);
        };
        let Some(Some(parent_tree)) = self.parent_tree.as_ref() else {
            return Ok(None);
        };

        let matcher = FilesetExpression::all().to_matcher();
        let mut copy_records = CopyRecords::default();
        for parent in commit.parent_ids() {
            let records = get_copy_records(store, parent, commit.id(), &matcher)?;
            copy_records.add_records(records)?;
        }
        let tree_diff = parent_tree.diff_stream_with_copies(tree, &matcher, &copy_records);
        let stats = DiffStats::calculate(
            repo.store(),
            tree_diff,
            &DiffStatOptions::default(),
            jj_lib::conflicts::ConflictMarkerStyle::Diff,
        )
        .block_on()?;

        Ok(Some(stats))
    }

    pub fn commit_is_empty(&mut self, command_helper: &CommandHelper) -> Result<Option<bool>> {
        self.load_parent_tree(command_helper)?;
        self.load_tree(command_helper)?;

        let Some(Some(tree)) = self.tree.as_ref() else {
            return Ok(None);
        };
        let Some(Some(parent_tree)) = self.parent_tree.as_ref() else {
            return Ok(None);
        };

        Ok(Some(tree == parent_tree))
    }
}
