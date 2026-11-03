"""Fetch the pinned Snowflake documentation pages into a private cache.

Usage: python -m scripts.syntax_catalogue.fetch CACHE_DIR INDEX_FILE...

INDEX_FILE are the documentation llms.txt indexes (SQL commands, functions, general reference,
classes). Pages are stored gzip-compressed under CACHE_DIR/raw with one line per fetch in
CACHE_DIR/fetch-log.jsonl (URL, UTC time, status, sha256). The cache is private and untracked.
"""

import concurrent.futures as cf
import datetime
import gzip
import hashlib
import json
import pathlib
import re
import sys
import time
import urllib.request

root = pathlib.Path(sys.argv[1])
idx_files = sys.argv[2:]
urls = []
for f in idx_files:
    for u in re.findall(
        r"https://docs\.snowflake\.com/en/[^) \s]*\.md", pathlib.Path(f).read_text()
    ):
        if u not in urls:
            urls.append(u)
log_path = root / "fetch-log.jsonl"
done = set()
if log_path.exists():
    for line in log_path.read_text().splitlines():
        r = json.loads(line)
        if r["status"] == 200:
            done.add(r["url"])
todo = [u for u in urls if u not in done]
print(len(urls), "urls", len(todo), "to fetch", flush=True)


def get(u):
    for attempt in range(4):
        try:
            req = urllib.request.Request(u, headers={"User-Agent": "sqlquack-syntax-catalogue/1"})
            with urllib.request.urlopen(req, timeout=40) as r:
                body = r.read()
                status = r.status
            break
        except urllib.error.HTTPError as e:
            body, status = b"", e.code
            if e.code == 404:
                break
        except Exception as e:
            body, status = b"", str(e)[:80]
        time.sleep(2 * (attempt + 1))
    path = u.split("/en/", 1)[1]
    if status == 200:
        out = root / "raw" / (path + ".gz")
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(gzip.compress(body))
    return {
        "url": u,
        "path": path,
        "status": status,
        "retrieved_utc": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "sha256": hashlib.sha256(body).hexdigest() if status == 200 else None,
        "bytes": len(body),
    }


with open(log_path, "a") as log, cf.ThreadPoolExecutor(6) as ex:
    n = 0
    for rec in ex.map(get, todo):
        log.write(json.dumps(rec) + "\n")
        n += 1
        if n % 200 == 0:
            log.flush()
            print(n, flush=True)
print("done", flush=True)
