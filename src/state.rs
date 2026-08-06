use std::sync::Arc;

use jj_cli::{
    cli_util::{CommandHelper, WorkspaceCommandHelper},
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
};
use pollster::FutureExt;

type Result<T> = std::result::Result<T, CommandError>;

pub struct State {
    snapshot: bool,
    workspace_helper: Option<WorkspaceCommandHelper>,
    repo: Option<Arc<ReadonlyRepo>>,
    commit_id: Option<Option<CommitId>>,
    commit: Option<Option<Commit>>,
    parent_commits: Option<Vec<Commit>>,
    tree: Option<Option<MergedTree>>,
    parent_tree: Option<Option<MergedTree>>,
}

impl State {
    pub fn new(snapshot: bool) -> Self {
        Self {
            snapshot,
            workspace_helper: Default::default(),
            repo: Default::default(),
            commit_id: Default::default(),
            commit: Default::default(),
            parent_commits: Default::default(),
            tree: Default::default(),
            parent_tree: Default::default(),
        }
    }

    fn load_workspace(&mut self, command_helper: &CommandHelper) -> Result<()> {
        if self.workspace_helper.is_some() {
            return Ok(());
        }
        let helper = if self.snapshot {
            command_helper.workspace_helper(&Ui::null()).block_on()?
        } else {
            command_helper
                .workspace_helper_no_snapshot(&Ui::null())
                .block_on()?
        };
        self.workspace_helper = Some(helper);
        Ok(())
    }

    pub fn workspace_helper(
        &mut self,
        command_helper: &CommandHelper,
    ) -> Result<&WorkspaceCommandHelper> {
        self.load_workspace(command_helper)?;
        let Some(w) = self.workspace_helper.as_ref() else {
            unreachable!()
        };
        Ok(w)
    }

    pub fn load_repo(&mut self, command_helper: &CommandHelper) -> Result<()> {
        if self.repo.is_some() {
            return Ok(());
        }
        let workspace_helper = self.workspace_helper(command_helper)?;
        let repo = workspace_helper.repo();
        self.repo = Some(repo.clone());
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
            .get_wc_commit_id(self.workspace_helper(command_helper)?.workspace_name())
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

    pub fn load_parent_commits(&mut self, command_helper: &CommandHelper) -> Result<()> {
        if self.parent_commits.is_some() {
            return Ok(());
        }

        let parent_commits = self
            .commit(command_helper)?
            .as_ref()
            .map(|commit| {
                let store = commit.store();
                commit
                    .parent_ids()
                    .iter()
                    .map(|id| store.get_commit(id))
                    .collect::<std::result::Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();

        self.parent_commits = Some(parent_commits);
        Ok(())
    }

    pub fn parent_commits(&mut self, command_helper: &CommandHelper) -> Result<&Vec<Commit>> {
        self.load_parent_commits(command_helper)?;
        let Some(w) = self.parent_commits.as_ref() else {
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
            .map(|c| c.parent_tree(repo.as_ref()).block_on())
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
        let tree = commit.as_ref().map(|c| c.tree());
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
            let records = get_copy_records(store, parent, commit.id(), &matcher).block_on()?;
            copy_records.add_records(records);
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

        Ok(Some(tree.tree_ids() == parent_tree.tree_ids()))
    }
}
