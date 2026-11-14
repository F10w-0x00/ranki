#!/usr/bin/env python3
import sys
import os
import re

def check_symbols(binary_path):
    print(f"=== Checking GLIBC Version Symbols for {binary_path} ===")
    
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
                print(f"  Library: {verneed_entry.name}")
                for vernaux in num_versions:
                    print(f"    Requires: {vernaux.name}")
                    m = re.match(r'GLIBC_(\d+)\.(\d+)', vernaux.name)
                    if m:
                        versions.append((int(m.group(1)), int(m.group(2))))
                        
            if not versions:
                print("[PASS] No GLIBC versions required.")
                return True
                
            max_maj, max_min = max(versions)
            print(f"\nHighest GLIBC symbol version required: GLIBC_{max_maj}.{max_min}")
            
            violating = [(maj, minor) for maj, minor in versions if maj > 2 or (maj == 2 and minor > 20)]
            if violating:
                print(f"[FAIL] Violating versions exceeding GLIBC_2.20 gate: {violating}")
                return False
            else:
                print(f"[PASS] All GLIBC symbols satisfy <= GLIBC_2.20 ceiling (Highest: GLIBC_{max_maj}.{max_min})")
                return True
    except ImportError:
        pass

    # Fallback to readelf command
    import subprocess
    res = subprocess.run(["readelf", "-V", binary_path], capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Error running readelf: {res.stderr}")
        return False
        
    output = res.stdout
    print(output)
    glibc_versions = re.findall(r'GLIBC_(\d+)\.(\d+)', output)
    if not glibc_versions:
        print("[WARNING] No GLIBC versions found in .gnu.version_r!")
        return True
        
    versions = [(int(a), int(b)) for a, b in glibc_versions]
    max_maj, max_min = max(versions)
    print(f"Highest GLIBC symbol version required: GLIBC_{max_maj}.{max_min}")
    
    violating = [f"GLIBC_{a}.{b}" for a, b in versions if a > 2 or (a == 2 and b > 20)]
    if violating:
        print(f"[FAIL] Violating symbols found exceeding GLIBC_2.20 gate: {violating}")
        return False
    else:
        print(f"[PASS] All GLIBC symbols satisfy <= GLIBC_2.20 ceiling (Highest: GLIBC_{max_maj}.{max_min})")
        return True

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: check_symbols.py <binary>")
        sys.exit(1)
    ok = check_symbols(sys.argv[1])
    sys.exit(0 if ok else 1)
