# PR workflow

Use the [commit](./commit.md) command if there are any pending changes.

Use the git and gh cli tools to fetch the diff between origin/main and the
current branch. Generate a concise summary of the content and purpose of these
changes based on the observed diff.

If some $ARGUMENTS are given, add to the summary "Close $ARGUMENTS". Use the
Linear MCP to fetch the corresponding $ARGUMENTS issue and make sure that the PR
content matches the issue description.

Wait for few minutes after the PR has been created to find a comment from Claude
making a detailed review of the PR. If the recommandation is not to approve,
work on the suggested improvements.
