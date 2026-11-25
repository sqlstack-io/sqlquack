"""Where the website shows examples, how a page marks them, and how a shown block reads."""

from __future__ import annotations

import re

HOMEPAGE: str = "website/public/index.html"
DOCS_DIR: str = "website/src/content/docs/docs"
EXAMPLES_DIR: str = "website/examples"
NO_INPUTS: str = "no-inputs"
ELISION: str = "…"
PROMPT: str = "$ "
PYTHON_KIND: str = "python"
FILE_KEY: str = "file="
PREAMBLE_KEY: str = "preamble="
OUTPUT_KEY: str = "output: "
SKIPPED_COMMANDS: tuple[str, ...] = ("pip install sqlquack",)
COMMAND_TIMEOUT_SECONDS: int = 60
MDX_BLOCK: re.Pattern[str] = re.compile(
    r"^(?:\{/\* example: (?P<spec>[^*\n]+?) \*/\}\n)?```(?P<language>\w*)[^\n]*\n(?P<body>.*?)^```",
    re.MULTILINE | re.DOTALL,
)
MDX_TRANSCRIPT_LANGUAGE: str = "console"
TERMINAL_BLOCK: re.Pattern[str] = re.compile(
    r'<pre class="terminal"(?: data-example="(?P<spec>[^"]+)")?>(?P<body>.*?)</pre>', re.DOTALL
)
COMMAND_SPAN: re.Pattern[str] = re.compile(r'<span class="cmd">(.*?)</span>', re.DOTALL)
TAG: re.Pattern[str] = re.compile(r"<[^>]+>")
PROGRESS_LINE: re.Pattern[str] = re.compile(r"sqlquack \w+: \w+ing\b[^\n]*…")
RESULT_COMMENT: re.Pattern[str] = re.compile(r"^(?P<expression>\S.*?)\s+# (?P<value>.+)$")
