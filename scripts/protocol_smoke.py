"""Exercise framing, error recovery, reusable process and clean shutdown."""
import json
import os
import subprocess

binary = os.environ.get('RDLP_BIN', '/tmp/r-dlp')
process = subprocess.Popen([binary, 'serve'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
assert json.loads(process.stdout.readline())['ready']
for line in ['invalid-json', '{"id":"--help"}', '{"id":"dQw4w9WgXcQ"}', '{"id":"M7lc1UVf-VE"}']:
    process.stdin.write(line+'\n'); process.stdin.flush()
    response = json.loads(process.stdout.readline())
    if line.startswith('{"id":"dQ') or line.startswith('{"id":"M7'):
        assert response.get('url', '').startswith('https://') and response['format_id'] == 140, response.get('error')
    else:
        assert response.get('error')
process.stdin.close()
assert process.wait(timeout=5) == 0
oversized = subprocess.run([binary,'serve'],input='x'*4097+'\n',capture_output=True,text=True,timeout=5)
assert oversized.returncode == 1
print('PASS: readiness, invalid JSON recovery, invalid ID recovery, two real resolutions, EOF and oversized frame rejection')
