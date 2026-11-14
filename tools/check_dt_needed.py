#!/usr/bin/env python3
import sys
import subprocess
import re

DEVICE_WHITELIST = {
    "libgtk-x11-2.0.so.0",
    "libgdk-x11-2.0.so.0",
    "libgdk_pixbuf-2.0.so.0",
    "libpangocairo-1.0.so.0",
    "libatk-1.0.so.0",
    "libcairo.so.2",
    "libpangoft2-1.0.so.0",
    "libpango-1.0.so.0",
    "libfreetype.so.6",
    "libfontconfig.so.1",
    "libgmodule-2.0.so.0",
    "libsoup-2.4.so.1",
    "libgio-2.0.so.0",
    "libgobject-2.0.so.0",
    "libgthread-2.0.so.0",
    "libglib-2.0.so.0",
    "libwebkitgtk-1.0.so",
    "libwebkitgtk-1.0.so.0",
    "libz.so.1",
    "liblipc.so.1",
    "libcurl.so.4",
    "libm.so.6",
    "libdl.so.2",
    "libgcc_s.so.1",
    "libpthread.so.0",
    "libc.so.6",
    "ld-linux.so.3"
}

def check_dt_needed(binary_path):
    print(f"=== Checking DT_NEEDED Whitelist for {binary_path} ===")
    
    needed = []
    try:
        from elftools.elf.elffile import ELFFile
        with open(binary_path, 'rb') as f:
            elf = ELFFile(f)
            dyn = elf.get_section_by_name('.dynamic')
            if dyn:
                for tag in dyn.iter_tags():
                    if tag.entry.d_tag == 'DT_NEEDED':
                        needed.append(tag.needed)
    except ImportError:
        res = subprocess.run(["readelf", "-d", binary_path], capture_output=True, text=True)
        if res.returncode != 0:
            print(f"Error running readelf: {res.stderr}")
            return False
        for line in res.stdout.splitlines():
            m = re.search(r'\(NEEDED\)\s+Shared library:\s+\[(.*?)\]', line)
            if m:
                needed.append(m.group(1))

    print(f"Discovered DT_NEEDED libraries ({len(needed)}):")
    for lib in needed:
        print(f"  {lib}")
        
    invalid = [lib for lib in needed if lib not in DEVICE_WHITELIST]
    if invalid:
        print(f"\n[FAIL] Foreign libraries not available on Kindle OS detected: {invalid}")
        return False
    else:
        print(f"\n[PASS] All {len(needed)} DT_NEEDED libraries are present in Kindle OS whitelist!")
        return True

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: check_dt_needed.py <binary>")
        sys.exit(1)
    ok = check_dt_needed(sys.argv[1])
    sys.exit(0 if ok else 1)
