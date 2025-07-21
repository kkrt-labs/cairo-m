# PR workflow

Make sure that `trunk check --ci` pass, if not, run `trunk fmt`, then commit
changes. Use the git and gh cli tools to fetch the diff between origin/main and
the current branch. Generate a concise summary of the content and purpose of
these changes based on the observed diff. Add to the summary "Close $ARGUMENTS".
Use the Linear MCP to fetch the corresponding $ARGUMENTS issue and make ure that
the PR content matches the issue description. Wait for few minutes after the Pr
has been created to find a comment from Claude making a detailed review of the
PR. If the recommandation is not to approve, work on the suggested improvements.
