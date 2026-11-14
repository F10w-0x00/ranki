#!/usr/bin/env python3
import sys
import os
import re
import subprocess

def check_symbols(binary_path):
    print("=== Checking GLIBC Version Symbols for {} ===".format(binary_path))
    
    # Try pyelftools first if available
    try:
        from elftools.elf.elffile import ELFFile
        with open(binary_path, 'rb') as f:
            elf = ELFFile(f)
            verneed = elf.get_section_by_name('.gnu.version_r')
            if not verneed:
                print("[WARNING] No .gnu.version_r section found!")
                return True
                
            versions = []
            for verneed_entry, num_versions in verneed.iter_versions():
                print("  Library: {}".format(verneed_entry.name))
                for vernaux in num_versions:
                    print("    Requires: {}".format(vernaux.name))
                    m = re.match(r'GLIBC_(\d+)\.(\d+)', vernaux.name)
                    if m:
                        versions.append((int(m.group(1)), int(m.group(2))))
                        
            if not versions:
                print("[PASS] No GLIBC versions required.")
                return True
                
            max_maj, max_min = max(versions)
            print("\nHighest GLIBC symbol version required: GLIBC_{}.{}".format(max_maj, max_min))
            
            violating = [(maj, minor) for maj, minor in versions if maj > 2 or (maj == 2 and minor > 20)]
            if violating:
                print("[FAIL] Violating versions exceeding GLIBC_2.20 gate: {}".format(violating))
                return False
            else:
                print("[PASS] All GLIBC symbols satisfy <= GLIBC_2.20 ceiling (Highest: GLIBC_{}.{})".format(max_maj, max_min))
                return True
    except ImportError:
        pass

    # Fallback to readelf command
    p = subprocess.Popen(["readelf", "-V", binary_path], stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True)
    out, err = p.communicate()
    if p.returncode != 0:
        print("Error running readelf: {}".format(err))
        return False
        
    glibc_versions = re.findall(r'GLIBC_(\d+)\.(\d+)', out)
    if not glibc_versions:
        print("[WARNING] No GLIBC versions found in .gnu.version_r!")
        return True
        
    versions = [(int(a), int(b)) for a, b in glibc_versions]
    max_maj, max_min = max(versions)
    print("Highest GLIBC symbol version required: GLIBC_{}.{}".format(max_maj, max_min))
    
    violating = ["GLIBC_{}.{}".format(a, b) for a, b in versions if a > 2 or (a == 2 and b > 20)]
    if violating:
        print("[FAIL] Violating symbols found exceeding GLIBC_2.20 gate: {}".format(violating))
        return False
    else:
        print("[PASS] All GLIBC symbols satisfy <= GLIBC_2.20 ceiling (Highest: GLIBC_{}.{})".format(max_maj, max_min))
        return True

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: check_symbols.py <binary>")
        sys.exit(1)
    ok = check_symbols(sys.argv[1])
    sys.exit(0 if ok else 1)
