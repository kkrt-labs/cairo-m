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
   - use the [commit](./commit.md) command for your commits
5. run `trunk fmt` for linter
6. use the [pr](./pr.md) command for creating the PR
