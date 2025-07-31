# PR workflow

Make sure that you have access to the Linear MCP. If not, break and ask for the
user to add it.

If can be added locally with this line:

```bash
claude mcp add-json linear '{"command": "npx", "args": ["-y","mcp-remote","https://mcp.linear.app/sse"]}'
```

In all your communications, please remain as succinct as possible, removing all
superlative and useless words.

Then

1. check the issue on linear based on the $ARGUMENTS
2. review the issue body and context, and prepare a checklist
3. if required, ask for clarification before doing any dev
4. then implement the feature:
   - create a branch with $ARGUMENTS as name
   - create simple commit for each part of the implementation
   - make sure to have `cargo b` and `cargo t` pass at each commit
5. run `trunk fmt` for linter
6. use the git and gh cli tools to fetch the diff between origin/main and the
   current branch, and create a PR
7. generate a concise summary of the content and purpose of these changes based
   on the observed diff. Add to the summary "Close $ARGUMENTS".
8. the title of the PR should start with feat(prover/runner/vm) (pick the one
   corresponding to the current task)
9. Wait for few minutes after the Pr has been created to find a comment from
   Claude making a detailed review of the PR. If the recommandation is not to
   approve, work on the suggested improvements.
