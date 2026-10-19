"""Decode a short segment from each native result on the resolving machine."""
import json
import os
import subprocess

binary = os.environ.get('RDLP_BIN', '/tmp/r-dlp')
videos = ['dQw4w9WgXcQ', '9bZkp7q19f0', 'M7lc1UVf-VE', 'jNQXAC9IVRw']
worker = subprocess.Popen([binary,'serve'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
assert json.loads(worker.stdout.readline())['ready']
for video in videos:
    worker.stdin.write(json.dumps({'id':video})+'\n');worker.stdin.flush()
    info = json.loads(worker.stdout.readline())
    assert 'url' in info, info.get('error')
    result = subprocess.run(['ffmpeg','-nostdin','-v','error','-i',info['url'],'-t','0.25','-vn','-f','s16le','-acodec','pcm_s16le','-'],capture_output=True,timeout=30)
    assert result.returncode==0 and len(result.stdout)>1000, 'Media decode failed'
    print(json.dumps({'video':video,'format_id':info['format_id'],'decode_ok':True,'pcm_bytes':len(result.stdout)}),flush=True)
worker.stdin.close();assert worker.wait(timeout=5)==0
