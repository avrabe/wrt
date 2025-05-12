"""
PR Node: Creates or updates GitHub pull requests for wrt automation.
Summarizes validation results and attaches logs to the PR body.
"""
import os
import json
from ghapi.actions import gh_action
import markdown

def pr_node(state, env):
    repo  = os.environ["GITHUB_REPOSITORY"]
    token = os.environ["GH_TOKEN"]
    gh    = gh_action(token)
    branch = os.getenv("GITHUB_HEAD_REF", "auto/" + state["ticket"])

    body_md = markdown.markdown(
        f"""
### Automated result for ticket #{state['ticket']}

| Stage | Outcome |
|-------|---------|
| **Build (std)** | { '✅' if 'std_build'  in state['validate_passed'] else '❌' } |
| **Build (no_std+alloc)** | { '✅' if 'nostd_build' in state['validate_passed'] else '❌' } |
| **Tests** | { '✅' if 'test'       in state['validate_passed'] else '❌' } |
| **Clippy** | { '✅' if 'clippy'    in state['validate_passed'] else '❌' } |

<details><summary>Logs</summary>

```
{json.dumps(state['validate_failed'], indent=2)}
```

</details>
"""
    )

    title = f"feat({state['crate']}): auto update for ticket #{state['ticket']}"

    prs = gh.repos.list("pulls", repo=repo, head=f"{repo.split('/')[0]}:{branch}")
    if prs:
        gh.issues.update("issues", repo=repo, issue_number=prs[0].number, title=title, body=body_md)
    else:
        gh.pulls.create("pulls", repo=repo, title=title, head=branch, base="main", body=body_md, draft=True)
    return state 