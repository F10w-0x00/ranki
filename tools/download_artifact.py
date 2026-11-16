import urllib.request
import json
import subprocess
import zipfile
import os

p = subprocess.Popen(['git', 'credential', 'fill'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
out, err = p.communicate(input='protocol=https\nhost=github.com\n\n')
token = ''
for line in out.splitlines():
    if line.startswith('password='):
        token = line.split('=', 1)[1]

run_id = '43877290241'
url = f'https://api.github.com/repos/F10w-0x00/ranki/actions/runs/{run_id}/artifacts'
req = urllib.request.Request(url, headers={'User-Agent': 'Python', 'Authorization': f'Bearer {token}'})
with urllib.request.urlopen(req) as resp:
    data = json.loads(resp.read().decode('utf-8'))
    artifacts = data.get('artifacts', [])

for a in artifacts:
    if a['name'] == 'ranki-armel-m0':
        print(f"Downloading {a['name']} (id: {a['id']}, size: {a['size_in_bytes']} bytes)...")
        download_url = a['archive_download_url']
        
        class NoRedirect(urllib.request.HTTPRedirectHandler):
            def redirect_request(self, req, fp, code, msg, hdrs, newurl):
                return None
        
        opener = urllib.request.build_opener(NoRedirect)
        dl_req = urllib.request.Request(download_url, headers={'User-Agent': 'Python', 'Authorization': f'Bearer {token}'})
        try:
            resp = opener.open(dl_req)
            redirect_url = resp.getheader('Location')
        except urllib.error.HTTPError as e:
            redirect_url = e.headers.get('Location')
            
        if redirect_url:
            zip_path = 'build_m0.zip'
            req2 = urllib.request.Request(redirect_url, headers={'User-Agent': 'Python'})
            with urllib.request.urlopen(req2) as resp2, open(zip_path, 'wb') as f:
                f.write(resp2.read())
            print(f"Saved {zip_path}, extracting...")
            os.makedirs('build', exist_ok=True)
            with zipfile.ZipFile(zip_path, 'r') as z:
                z.extractall('build')
            print("Extracted build/ranki-armel successfully!")
            if os.path.exists('build_m0.zip'):
                os.remove('build_m0.zip')
