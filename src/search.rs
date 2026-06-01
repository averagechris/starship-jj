use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::Arc;

use jj_cli::command_error::CommandError;
use jj_lib::{backend::CommitId, store::Store, view::View};

use crate::config::{BookmarkConfig, IgnoreEmpty};

pub(crate) fn find_parent_bookmarks(
    commit_id: &CommitId,
    config: &BookmarkConfig,
    bookmarks: &mut BTreeMap<String, usize>,
    view: &View,
    store: &Arc<Store>,
    visited: &mut HashSet<CommitId>,
    ignore_empty_commits: IgnoreEmpty,
) -> Result<(), CommandError> {
    let mut queue: VecDeque<(CommitId, usize, usize, IgnoreEmpty)> = VecDeque::new();
    queue.push_back((commit_id.clone(), 0, 0, ignore_empty_commits));

    let mut best_distance: Option<usize> = bookmarks.values().min().copied();

    while let Some((cid, search_depth, distance, ignore_empty_commits)) = queue.pop_front() {
        if let Some(best) = best_distance
            && distance > best
        {
            continue;
        }
        if !visited.insert(cid.clone()) {
            continue;
        }

        let mut names = view
            .local_bookmarks_for_commit(&cid)
            .map(|(name, _)| name)
            .peekable();

        if names.peek().is_some() {
            let mut inserted_any = false;
            'bookmark: for bookmark in names {
                let bookmark = bookmark.as_str();
                for glob in &config.exclude {
                    #[cfg(not(feature = "json-schema"))]
                    if glob.matches(bookmark) {
                        continue 'bookmark;
                    }
                }
                let bookmark = bookmark.to_string();
                let mut inserted = false;
                bookmarks
                    .entry(bookmark)
                    .and_modify(|v| {
                        if *v > distance {
                            *v = distance;
                        }
                    })
                    .or_insert_with(|| {
                        inserted = true;
                        distance
                    });
                inserted_any |= inserted;
            }
            if inserted_any && best_distance.is_none_or(|best| distance < best) {
                best_distance = Some(distance);
            }
            continue;
        }

        if search_depth >= config.search_depth {
            continue;
        }
        if let Some(best) = best_distance
            && distance + 1 > best
        {
            continue;
        }

        let commit = store.get_commit(&cid)?;
        let ignore_current =
            ignore_empty_commits != IgnoreEmpty::None && commit.description().is_empty();
        let parent_distance = if ignore_current {
            distance
        } else {
            distance + 1
        };
        let parent_ignore_empty_commits = if ignore_empty_commits == IgnoreEmpty::Current {
            IgnoreEmpty::None
        } else {
            ignore_empty_commits
        };
        for p in commit.parent_ids() {
            queue.push_back((
                p.clone(),
                search_depth + 1,
                parent_distance,
                parent_ignore_empty_commits,
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet, VecDeque};

    // Pure BFS test harness, mirroring production behavior without jj types.
    fn bfs_traverse(
        start: &'static str,
        search_depth: usize,
        graph: &HashMap<&'static str, Vec<&'static str>>, // child -> parents
        marks: &HashMap<&'static str, Vec<&'static str>>, // commit -> bookmark names
        exclude: impl Fn(&str) -> bool,
    ) -> (BTreeMap<String, usize>, HashSet<&'static str>) {
        let mut out: BTreeMap<String, usize> = BTreeMap::new();
        let mut visited: HashSet<&'static str> = HashSet::new();
        let mut q: VecDeque<(&'static str, usize)> = VecDeque::new();
        let mut best_depth: Option<usize> = None;
        q.push_back((start, 0));

        while let Some((id, d)) = q.pop_front() {
            if let Some(best) = best_depth
                && d > best
            {
                break;
            }
            if !visited.insert(id) {
                continue;
            }

            if let Some(names) = marks.get(id) {
                let mut inserted_any = false;
                for &name in names {
                    if exclude(name) {
                        continue;
                    }
                    let mut inserted = false;
                    out.entry(name.to_string())
                        .and_modify(|v| {
                            if *v > d {
                                *v = d;
                            }
                        })
                        .or_insert_with(|| {
                            inserted = true;
                            d
                        });
                    if inserted {
                        inserted_any = true;
                    }
                }
                if inserted_any && best_depth.is_none_or(|best| d < best) {
                    best_depth = Some(d);
                }
                continue;
            }

            if d >= search_depth {
                continue;
            }
            if let Some(best) = best_depth
                && d + 1 > best
            {
                continue;
            }

            if let Some(parents) = graph.get(id) {
                for &p in parents {
                    q.push_back((p, d + 1));
                }
            }
        }

        (out, visited)
    }

    #[test]
    fn bfs_finds_all_min_depth_and_stops_deeper() {
        use std::collections::HashMap;
        let graph: HashMap<_, _> = HashMap::from([
            ("A", vec!["B", "C"]),
            ("B", vec!["D"]),
            ("C", vec![]),
            ("D", vec![]),
        ]);
        let marks: HashMap<_, _> =
            HashMap::from([("B", vec!["b"]), ("C", vec!["c"]), ("D", vec!["d"])]);
        let (out, visited) = bfs_traverse("A", 10, &graph, &marks, |_| false);
        assert_eq!(
            out,
            BTreeMap::from([(String::from("b"), 1), (String::from("c"), 1),])
        );
        assert!(visited.contains("B") && visited.contains("C"));
        assert!(!visited.contains("D"));
    }

    #[test]
    fn bfs_stops_other_branch_after_near_bookmark() {
        use std::collections::HashMap;
        let graph: HashMap<_, _> = HashMap::from([
            ("A", vec!["C", "B"]),
            ("B", vec!["D"]),
            ("C", vec![]),
            ("D", vec![]),
        ]);
        let marks: HashMap<_, _> = HashMap::from([("C", vec!["x"]), ("D", vec!["y"])]);
        let (out, visited) = bfs_traverse("A", 10, &graph, &marks, |_| false);
        assert_eq!(out, BTreeMap::from([(String::from("x"), 1)]));
        assert!(visited.contains("B"));
        assert!(!visited.contains("D"));
    }

    #[test]
    fn bfs_obeys_search_depth_cutoff() {
        use std::collections::HashMap;
        let graph: HashMap<_, _> =
            HashMap::from([("A", vec!["B"]), ("B", vec!["C"]), ("C", vec![])]);
        let marks: HashMap<_, _> = HashMap::from([("C", vec!["far"])]);
        let (out, _visited) = bfs_traverse("A", 1, &graph, &marks, |_| false);
        assert!(out.is_empty());
    }

    #[test]
    fn bfs_excluded_only_commit_blocks_descending() {
        use std::collections::HashMap;
        let graph: HashMap<_, _> =
            HashMap::from([("A", vec!["B"]), ("B", vec!["C"]), ("C", vec![])]);
        let marks: HashMap<_, _> = HashMap::from([("B", vec!["r/blocked"]), ("C", vec!["ok"])]);
        let (out, visited) = bfs_traverse("A", 10, &graph, &marks, |name| name.starts_with("r/"));
        assert!(out.is_empty());
        assert!(visited.contains("B"));
        assert!(!visited.contains("C"));
    }
}
