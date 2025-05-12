"""
Plan Node: Multi-backend planning node for wrt automation.
Supports Anthropic, OpenAI cloud, or local LLM endpoints.
Generates a plan (list of tasks) for a given ticket/issue.
"""
import os, json
from typing import Dict
from ghapi.actions import gh_action

PROVIDER = os.getenv("LLM_PROVIDER", "anthropic").lower()

if PROVIDER == "anthropic":
    from anthropic import Anthropic, HUMAN_PROMPT, AI_PROMPT
    _cli = Anthropic(api_key=os.getenv("CLAUDE_KEY"))
    def _chat(prompt: str) -> str:
        return _cli.completions.create(
            model=os.getenv("CLAUDE_MODEL", "claude-3-haiku-20240307"),
            prompt=f"{HUMAN_PROMPT} {prompt}{AI_PROMPT}",
            max_tokens_to_sample=512,
        ).completion
else:
    import openai
    if PROVIDER == "local":
        openai.base_url = os.getenv("OPENAI_BASE_URL", "http://localhost:11434/v1")
        openai.api_key  = "sk-local"
    else:
        openai.api_key = os.getenv("OPENAI_API_KEY")
    MODEL = os.getenv("OPENAI_MODEL", "gpt-4o-mini")
    def _chat(prompt: str) -> str:
        rsp = openai.chat.completions.create(
            model=MODEL,
            messages=[{"role": "user", "content": prompt}],
            max_tokens=512,
        )
        return rsp.choices[0].message.content

PROMPT = (
    "You are the planning node for **wrt**. Output JSON {\"tasks\": [...]}"\
)

def _body(issue: str) -> str:
    repo = os.getenv("GITHUB_REPOSITORY", "")
    tok  = os.getenv("GH_TOKEN")
    if repo and tok:
        gh = gh_action(tok)
        return gh.repos.get("issues", repo=repo, issue_number=int(issue)).body or ""
    return "(local)"

def plan_node(state: Dict, env):
    body = _body(state.get("ticket", "0"))
    try:
        tasks = json.loads(_chat(PROMPT + "\n\n" + body))["tasks"]
    except Exception:
        tasks = ["(LLM failed)"]
    return {**state, "plan": tasks} 