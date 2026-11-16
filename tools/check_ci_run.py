import urllib.request
import json
import sys
import subprocess

p = subprocess.Popen(['git', 'credential', 'fill'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
out, err = p.communicate(input='protocol=https\nhost=github.com\n\n')
token = ''
for line in out.splitlines():
    if line.startswith('password='):
        token = line.split('=', 1)[1]

headers = {'User-Agent': 'Python'}
if token:
    headers['Authorization'] = f'Bearer {token}'

if len(sys.argv) > 1:
    run_id = sys.argv[1]
else:
    list_url = 'https://api.github.com/repos/F10w-0x00/ranki/actions/runs?per_page=1'
    req0 = urllib.request.Request(list_url, headers=headers)
    with urllib.request.urlopen(req0) as resp0:
        data0 = json.loads(resp0.read().decode('utf-8'))
        run_id = str(data0['workflow_runs'][0]['id'])

print(f"Tracking run ID: {run_id}")
url = f'https://api.github.com/repos/F10w-0x00/ranki/actions/runs/{run_id}/jobs'
req = urllib.request.Request(url, headers=headers)
try:
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode('utf-8'))
        for j in data.get('jobs', []):
            print(f"Job: {j['name']} - status: {j['status']}, conclusion: {j['conclusion']}")
            for s in j.get('steps', []):
                print(f"  Step: {s['name']} -> {s['status']} ({s['conclusion']})")
except Exception as e:
    print('Error:', e)
