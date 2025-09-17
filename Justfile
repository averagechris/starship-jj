# https://github.com/casey/just
.PHONY: upstream rebase-upstream

# Fetch from upstream and update local tracking bookmarks
upstream:
	jj git fetch --remote upstream

# Rebase our main stack onto upstream/main, keeping our changes on top
rebase-upstream:
	jj rebase -b main -d main@upstream
