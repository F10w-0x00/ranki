import urllib.request
import json
import sys

run_id = sys.argv[1] if len(sys.argv) > 1 else '43877290241'
url = f'https://api.github.com/repos/F10w-0x00/ranki/actions/runs/{run_id}/jobs'
req = urllib.request.Request(url, headers={'User-Agent': 'Python'})
try:
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode('utf-8'))
        for j in data.get('jobs', []):
            print(f"Job: {j['name']} - status: {j['status']}, conclusion: {j['conclusion']}")
            for s in j.get('steps', []):
                print(f"  Step: {s['name']} -> {s['status']} ({s['conclusion']})")
except Exception as e:
    print('Error:', e)
