#!/usr/bin/env python3
import sys
import subprocess
import re

def check_dynsym(binary_path):
    print("=== Checking .dynsym Symbol Isolation for {} ===".format(binary_path))

    # Forbidden symbols that MUST NOT be exported by the main binary:
    # 1. Any sqlite3_* symbols (must remain hidden in librslib.a to prevent heap collision with Kindle OS libsoup)
    # 2. Any archive_* symbols (must remain hidden in libarchive.a)
    # 3. Any json_* symbols (must remain hidden in libjson-glib-1.0.a)
    FORBIDDEN_PREFIXES = ["sqlite3_", "archive_", "json_"]

    leaked = []

    try:
        from elftools.elf.elffile import ELFFile
        with open(binary_path, 'rb') as f:
            elf = ELFFile(f)
            dynsym = elf.get_section_by_name('.dynsym')
            if not dynsym:
                print("[WARNING] No .dynsym section found in binary.")
                return True

            for sym in dynsym.iter_symbols():
                # We only care about symbols defined in the binary (st_shndx != 'SHN_UNDEF')
                # and exported with GLOBAL or WEAK binding
                bind = sym['st_info']['bind']
                shndx = sym['st_shndx']
                if shndx != 'SHN_UNDEF' and bind in ('STB_GLOBAL', 'STB_WEAK'):
                    for prefix in FORBIDDEN_PREFIXES:
                        if sym.name.startswith(prefix):
                            leaked.append((sym.name, bind, shndx))

    except ImportError:
        # Fallback to readelf --dyn-syms
        p = subprocess.Popen(["readelf", "--dyn-syms", "-W", binary_path],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True)
        out, err = p.communicate()
        if p.returncode != 0:
            print("[ERROR] Failed to run readelf: {}".format(err))
            return False

        for line in out.splitlines():
            parts = line.strip().split()
            # Standard readelf --dyn-syms format:
            # Num: Value Size Type Bind Vis Ndx Name
            if len(parts) >= 8 and parts[0].endswith(':'):
                bind = parts[4]
                ndx = parts[6]
                name = parts[7]
                if ndx != 'UND' and bind in ('GLOBAL', 'WEAK'):
                    for prefix in FORBIDDEN_PREFIXES:
                        if name.startswith(prefix):
                            leaked.append((name, bind, ndx))

    if leaked:
        print("[FAIL] STATIC SYMBOL LEAKAGE DETECTED in .dynsym! Total offending symbols: {}".format(len(leaked)))
        print("  Offending exported symbols:")
        for name, bind, ndx in leaked[:20]:
            print("    {} (bind: {}, ndx: {})".format(name, bind, ndx))
        if len(leaked) > 20:
            print("    ... and {} more symbols.".format(len(leaked) - 20))
        print("\n[REMEDY] Ensure linker flags contain: -Wl,--exclude-libs,librslib.a (or -Wl,--exclude-libs,ALL)")
        return False
    else:
        print("[PASS] 0 forbidden static library symbols exported into .dynsym. Symbol isolation verified!")
        return True

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: check_dynsym.py <binary>")
        sys.exit(1)
    ok = check_dynsym(sys.argv[1])
    sys.exit(0 if ok else 1)
