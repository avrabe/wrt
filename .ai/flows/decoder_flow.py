import os, tomli, pathlib
from .base_dag import workflow
CFG = tomli.loads((pathlib.Path(__file__).parent.parent / "config/ai.toml").read_text())
PROFILE = "decoder"
state = {"ticket": os.getenv("TICKET", "local"), **CFG[PROFILE]}
workflow.invoke(state) 