#!/usr/bin/env python3
"""
tools/download_artifact.py
Downloads the latest CI-built ranki-armel artifact from GitHub Actions.
"""

import urllib.request
import json
import subprocess
import zipfile
import sys
import os


def get_github_token():
    p = subprocess.Popen(
        ["git", "credential", "fill"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    out, _ = p.communicate(input="protocol=https\nhost=github.com\n\n")
    for line in out.splitlines():
        if line.startswith("password="):
            return line.split("=", 1)[1].strip()
    return ""


def main():
    token = get_github_token()
    headers = {"User-Agent": "Python"}
    if token:
        headers["Authorization"] = f"Bearer {token}"

    if len(sys.argv) > 1:
        run_id = sys.argv[1]
    else:
        list_url = "https://api.github.com/repos/F10w-0x00/ranki/actions/runs?per_page=1"
        req0 = urllib.request.Request(list_url, headers=headers)
        with urllib.request.urlopen(req0) as resp0:
            data0 = json.loads(resp0.read().decode("utf-8"))
            run_id = str(data0["workflow_runs"][0]["id"])

    print(f"Downloading artifact from run ID: {run_id}")
    url = f"https://api.github.com/repos/F10w-0x00/ranki/actions/runs/{run_id}/artifacts"
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        artifacts = data.get("artifacts", [])

    preferred = ("ranki-armel", "ranki-armel-m0")
    selected = None
    for wanted in preferred:
        for a in artifacts:
            if a["name"] == wanted:
                selected = a
                break
        if selected:
            break

    if not selected:
        names = [a["name"] for a in artifacts]
        print(f"[ERROR] No ranki-armel artifact found in run {run_id}. Available: {names}")
        sys.exit(1)

    print(
        f"Downloading {selected['name']} (id: {selected['id']}, size: {selected['size_in_bytes']} bytes)..."
    )
    download_url = selected["archive_download_url"]

    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, hdrs, newurl):
            return None

    opener = urllib.request.build_opener(NoRedirect)
    dl_req = urllib.request.Request(download_url, headers=headers)
    try:
        resp = opener.open(dl_req)
        redirect_url = resp.getheader("Location")
    except urllib.error.HTTPError as e:
        redirect_url = e.headers.get("Location")

    if not redirect_url:
        print("[ERROR] Could not resolve artifact download redirect URL.")
        sys.exit(1)

    zip_path = "build_artifact.zip"
    req2 = urllib.request.Request(redirect_url, headers={"User-Agent": "Python"})
    with urllib.request.urlopen(req2) as resp2, open(zip_path, "wb") as f:
        f.write(resp2.read())

    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    build_dir = os.path.join(repo_root, "build")
    os.makedirs(build_dir, exist_ok=True)
    with zipfile.ZipFile(zip_path, "r") as z:
        z.extractall(build_dir)
    os.remove(zip_path)
    print(f"[PASS] Extracted artifact to {os.path.join(build_dir, 'ranki-armel')}")


if __name__ == "__main__":
    main()
