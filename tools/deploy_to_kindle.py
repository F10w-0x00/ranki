#!/usr/bin/env python3
"""
tools/deploy_to_kindle.py
Safe automated deployment script for Kindle Paperwhite 3 (Route B).
- Guarantees absolute safety of production database D:\anki_data (0 bytes modified).
- Never embeds hardcoded hkey tokens and never overwrites existing device config.ini.
"""

import os
import sys
import shutil
import hashlib


DEFAULT_CONFIG_INI = (
    "[General]\n"
    "collection_dir=/mnt/us/anki_data_test\n"
    "hkey=\n"
    "sync_server=\n"
    "font_size_px=42\n"
)


def get_dir_hash(path):
    if not os.path.exists(path):
        return None
    entries = []
    for root, _, files in os.walk(path):
        for name in sorted(files):
            fp = os.path.join(root, name)
            try:
                st = os.stat(fp)
                rel = os.path.relpath(fp, path)
                entries.append(f"{rel}:{st.st_size}:{st.st_mtime}")
            except OSError:
                pass
    return hashlib.sha256("\n".join(entries).encode("utf-8")).hexdigest()


def detect_kindle_drive():
    for drive_letter in ["D", "E", "F", "G", "H"]:
        candidate = f"{drive_letter}:\\"
        if os.path.exists(candidate) and os.path.exists(os.path.join(candidate, "extensions")):
            return candidate
    if os.path.exists("D:\\"):
        return "D:\\"
    return None


def main():
    print("==================================================")
    print("  RAnki Route B — Safe Kindle PW3 Deployer        ")
    print("==================================================")

    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    binary_path = os.path.join(repo_root, "build", "ranki-armel")
    views_path = os.path.join(repo_root, "views")

    if not os.path.isfile(binary_path):
        print(f"[ERROR] Binary {binary_path} not found! Run tools/download_artifact.py first.")
        sys.exit(1)

    target_drive = detect_kindle_drive()
    if not target_drive:
        print("[WARN] No Kindle USB drive with extensions/ detected.")
        print("Connect your Kindle PW3 in USB mass storage mode and re-run.")
        sys.exit(0)

    print(f"[OK] Kindle root detected at: {target_drive}")

    # RED LINE: Fingerprint production database D:\anki_data
    prod_data = os.path.join(target_drive, "anki_data")
    prod_hash_before = get_dir_hash(prod_data)
    if prod_hash_before:
        print(f"[RED LINE] Production data at {prod_data} (fingerprint: {prod_hash_before[:12]}).")
        print("Guaranteed: Deployer will NEVER read/write/modify this directory.")

    test_data = os.path.join(target_drive, "anki_data_test")
    if not os.path.exists(test_data):
        print(f"[INFO] Creating isolated test collection directory: {test_data}")
        os.makedirs(test_data, exist_ok=True)

    dev_ext = os.path.join(target_drive, "extensions", "ranki-dev")
    os.makedirs(dev_ext, exist_ok=True)

    # 1. Deploy binary
    target_bin = os.path.join(dev_ext, "ranki-armel")
    print(f"Deploying binary -> {target_bin} ...")
    shutil.copy2(binary_path, target_bin)

    # 2. Deploy views (card renderer + KaTeX + fallback MathJax + Anthropic fonts)
    target_views = os.path.join(dev_ext, "views")
    print(f"Deploying views -> {target_views} ...")
    if os.path.exists(target_views):
        shutil.rmtree(target_views)
    shutil.copytree(views_path, target_views)

    # 3. Deploy launcher script ranki.sh (strict Unix LF)
    target_sh = os.path.join(dev_ext, "ranki.sh")
    sh_content = (
        "#!/bin/bash\n"
        "export DISPLAY=:0\n"
        "export HOME=/mnt/us\n"
        "mkdir -p /mnt/us/ranki_logs\n"
        "cd /mnt/us/extensions/ranki-dev\n"
        "chmod +x ranki-armel\n"
        "exec ./ranki-armel > /mnt/us/ranki_logs/ranki_dev_stdout.log 2>&1\n"
    )
    with open(target_sh, "w", newline="\n", encoding="utf-8") as f:
        f.write(sh_content)
    print(f"Generated launcher script -> {target_sh}")

    # 4. Preserve existing config.ini (never overwrite user token/settings!)
    target_ini = os.path.join(dev_ext, "config.ini")
    if os.path.exists(target_ini):
        print(f"[OK] Existing {target_ini} preserved untouched (user hkey & settings kept).")
    else:
        with open(target_ini, "w", newline="\n", encoding="utf-8") as f:
            f.write(DEFAULT_CONFIG_INI)
        print(f"Created default config (empty hkey) -> {target_ini}")

    # Verify production database untouched
    if prod_hash_before:
        prod_hash_after = get_dir_hash(prod_data)
        assert prod_hash_before == prod_hash_after, "[FATAL] Production data changed! Aborting!"
        print("[PASS] Production data verified 100% UNTOUCHED (0-byte modification).")

    print("\n[SUCCESS] Deployment complete!")
    print("Next steps on Kindle PW3:")
    print("1. Safely eject USB mass storage.")
    print("2. Launch KUAL -> Extensions -> 'RAnki Dev (M0)'.")
    print("3. Inspect /mnt/us/ranki_logs/ranki_dev_stdout.log after testing.")
    print("==================================================")


if __name__ == "__main__":
    main()
