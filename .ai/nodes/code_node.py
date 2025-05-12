"""
Code Node: Applies code changes for wrt automation.
Uses Anthropic Claude to generate and apply git diffs for a given plan.
Commits changes with context-aware commit messages.
"""
import os, pathlib, textwrap, re
from typing import Dict
from git import Repo
from anthropic import Anthropic, HUMAN_PROMPT, AI_PROMPT
from unidiff import PatchSet
from ghapi.actions import gh_action

CLIENT = Anthropic(api_key=os.getenv("CLAUDE_KEY"))

DIFF_PROMPT = textwrap.dedent("""
You are the code node for **wrt**. Given this plan:\n\n{plan}\n\nProduce a git unified diff limited to crate `{crate}`. Wrap in <diff> tags.
""")

def _apply(patch: str):
    repo = Repo(".")
    for pf in PatchSet(patch):
        path = pathlib.Path(pf.path)
        if pf.is_removed_file:
            path.unlink(missing_ok=True); continue
        path.parent.mkdir(parents=True, exist_ok=True)
        txt = path.read_text() if path.exists() else ""
        lines = txt.splitlines(keepends=True)
        for h in pf:
            lines[h.source_start-1:h.source_start-1+h.source_length] = [l.value for l in h]
        path.write_text("".join(lines))
    repo.git.add(all=True)


def _commit(repo: Repo, crate: str, plan):
    label2type = {"feat":"feat","fix":"fix","bug":"fix","refactor":"refactor","docs":"docs","test":"test"}
    gh = gh_action(os.getenv("GH_TOKEN")) if os.getenv("GH_TOKEN") else None
    ctype = "chore"
    if gh:
        issue = gh.repos.get("issues", repo=os.getenv("GITHUB_REPOSITORY"), issue_number=int(os.getenv("TICKET","0")))
        for lab in issue.labels:
            ctype = label2type.get(lab.name.lower(), ctype)
    subj = re.sub(r"[\r\n]", " ", plan[0])[:50]
    repo.index.commit(f"{ctype}({crate}): {subj}")

def code_node(state: Dict, env):
    plan  = state["plan"]
    crate = state["crate"]
    diff  = CLIENT.completions.create(
        model="claude-3-haiku-20240307",
        prompt=f"{HUMAN_PROMPT} {DIFF_PROMPT.format(plan='\n'.join(plan), crate=crate)}{AI_PROMPT}",
        max_tokens_to_sample=2048,
    ).completion
    diff = diff.split("<diff>")[-1].split("</diff>")[0]
    if diff.strip():
        _apply(diff); _commit(Repo("."), crate, plan)
    return state 