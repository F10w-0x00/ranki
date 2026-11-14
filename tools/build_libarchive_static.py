#!/usr/bin/env python3
import os
import subprocess
import urllib.request
import tarfile
import shutil

DEST_A = '/usr/lib/arm-linux-gnueabi/libarchive.a'

def needs_rebuild():
    if not os.path.exists(DEST_A):
        return True
    try:
        with open(DEST_A, 'rb') as f:
            content = f.read()
            if b'acl_init' in content or b'nettle_' in content:
                print("Existing libarchive.a has unwanted acl/nettle symbols, rebuilding minimal static version...")
                return True
    except Exception as e:
        print(f"Error checking {DEST_A}: {e}")
        return True
    return False

if not needs_rebuild():
    print(f"{DEST_A} already exists and is clean, skipping rebuild.")
    exit(0)

print("Downloading and compiling minimal static libarchive for Kindle OS target...")
url = 'https://github.com/libarchive/libarchive/releases/download/v3.3.3/libarchive-3.3.3.tar.gz'
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
build_dir = '/tmp/libarchive-build'
os.makedirs(build_dir, exist_ok=True)
tar_path = os.path.join(build_dir, 'libarchive-3.3.3.tar.gz')

with urllib.request.urlopen(req) as resp, open(tar_path, 'wb') as out_file:
    shutil.copyfileobj(resp, out_file)

with tarfile.open(tar_path, mode='r:gz') as tar:
    tar.extractall(build_dir)

src_dir = os.path.join(build_dir, 'libarchive-3.3.3')
env = os.environ.copy()
env['CC'] = 'arm-linux-gnueabi-gcc'
env['CFLAGS'] = '-O2 -I/usr/include'
env['LDFLAGS'] = '-L/usr/lib/arm-linux-gnueabi'

configure_cmd = [
    './configure',
    '--host=arm-linux-gnueabi',
    '--prefix=/usr',
    '--enable-static',
    '--disable-shared',
    '--disable-bsdtar',
    '--disable-bsdcpio',
    '--disable-bsdcat',
    '--disable-acl',
    '--disable-xattr',
    '--without-nettle',
    '--without-openssl',
    '--without-xml2',
    '--without-expat',
    '--without-bz2lib',
    '--without-lzma',
    '--without-zstd',
    '--without-lz4',
    '--without-cng',
    '--with-zlib'
]

print("Running ./configure for libarchive...")
subprocess.check_call(configure_cmd, cwd=src_dir, env=env)

nproc = os.cpu_count() or 2
print(f"Compiling libarchive with make -j{nproc}...")
subprocess.check_call(['make', f'-j{nproc}'], cwd=src_dir, env=env)

src_a = None
for root, dirs, files in os.walk(src_dir):
    if 'libarchive.a' in files:
        src_a = os.path.join(root, 'libarchive.a')
        print(f"Found built archive at: {src_a}")
        break

if not src_a:
    # Print all files for diagnosis
    all_files = []
    for root, dirs, files in os.walk(src_dir):
        for f in files:
            if f.endswith('.a') or f.endswith('.la'):
                all_files.append(os.path.join(root, f))
    raise RuntimeError(f"libarchive.a not found! Discovered archives: {all_files}")

shutil.copy(src_a, DEST_A)
size = os.path.getsize(DEST_A)
print(f"Successfully installed clean minimal static {DEST_A} ({size} bytes)")
