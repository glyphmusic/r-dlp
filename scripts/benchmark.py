"""Run inside the PC0373 companion's network/runtime. Does not restart it.
Outputs redacted evidence: no signed URLs, tokens or egress IP addresses.
Cold means a fresh process (not a flushed OS page cache). Warm means a
reused process/client, always a fresh extraction, never a result-cache hit.
"""
import contextlib
import datetime
import hashlib
import json
import os
import platform
import statistics
import subprocess
import sys
import time
import urllib.request

VIDEOS = ['dQw4w9WgXcQ', '9bZkp7q19f0', 'M7lc1UVf-VE', 'jNQXAC9IVRw']
FORMAT = 'bestaudio[ext=m4a]/bestaudio[acodec=vorbis]'
RUST = os.environ.get('RDLP_BIN', '/tmp/r-dlp')
ARGS = ['yt-dlp', '--ignore-config', '--no-warnings', '--no-playlist', '-f', FORMAT]

def probe(url):
    request = urllib.request.Request(url, headers={'Range': 'bytes=0-1023'})
    with urllib.request.urlopen(request, timeout=15) as response:
        data = response.read(1024)
        if response.status != 206 or len(data) != 1024:
            raise RuntimeError('Invalid range response')
        return {'status': response.status, 'bytes': len(data)}

def record(mode, video, action):
    start = time.perf_counter()
    row = {'mode': mode, 'video': video}
    try:
        audio = action()
        row['resolve_ms'] = (time.perf_counter() - start) * 1000
        row['format_id'] = str(audio.get('format_id', audio.get('format', '')))
        row['probe'] = probe(audio['url'])
        row['total_ms'] = (time.perf_counter() - start) * 1000
        row['ok'] = True
    except Exception as error:
        # Exception text may contain a signed URL; keep only its class.
        row.update(ok=False, error=type(error).__name__, elapsed_ms=(time.perf_counter()-start)*1000)
    print(json.dumps(row), flush=True)
    rows.append(row)

def rust_cold(video):
    result = subprocess.run([RUST, video], capture_output=True, text=True, timeout=35, check=True)
    return json.loads(result.stdout)

def python_cold(video):
    result = subprocess.run(ARGS + ['--print', '%(url)s\t%(format_id)s', '--', video], capture_output=True, text=True, timeout=60, check=True)
    url, format_id = result.stdout.strip().split('\t')
    return {'url': url, 'format_id': format_id}

rows = []
print(json.dumps({'environment': platform.platform(), 'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'yt_dlp': subprocess.check_output(['yt-dlp', '--version'], text=True).strip(),
 'r_dlp': subprocess.check_output([RUST, '--version'], text=True).strip(),
 'binary_sha256': hashlib.sha256(open(RUST,'rb').read()).hexdigest(),
 'rounds': 3, 'videos': VIDEOS, 'format': FORMAT,
 'cold_definition':'fresh process, OS cache not flushed',
 'warm_definition':'same process/client; fresh upstream extraction; no result cache'}), flush=True)

# Interleave implementations and reverse order each round to reduce timing bias.
for repeat in range(3):
    for video in VIDEOS:
        modes = [('yt-dlp-cold', python_cold), ('r-dlp-cold', rust_cold)]
        for name, action in (modes if repeat % 2 == 0 else list(reversed(modes))):
            record(name, video, lambda: action(video))

sys.path.insert(0, '/usr/local/bin/yt-dlp')
import yt_dlp
class Quiet:
    def debug(self, *_): pass
    def warning(self, *_): pass
    def error(self, *_): pass
options = yt_dlp.parse_options(ARGS[1:]).ydl_opts
options.update(quiet=True, logger=Quiet(), socket_timeout=15)

with yt_dlp.YoutubeDL(options) as python:
    rust = subprocess.Popen([RUST, 'serve'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    assert json.loads(rust.stdout.readline())['ready']
    def rust_warm(video):
        rust.stdin.write(json.dumps({'id': video}) + '\n'); rust.stdin.flush()
        result = json.loads(rust.stdout.readline())
        if 'error' in result: raise RuntimeError('Rust resolution failed')
        return result
    def python_warm(video):
        with contextlib.redirect_stdout(sys.stderr):
            info = python.extract_info('https://www.youtube.com/watch?v='+video, download=False)
        return {'url': info['url'], 'format_id': info['format_id']}
    # Explicit warmup, excluded from measured rows, same title for both.
    for action in [python_warm, rust_warm]:
        try: action(VIDEOS[0])
        except Exception: pass
    for repeat in range(3):
        for video in VIDEOS:
            modes = [('yt-dlp-warm', python_warm), ('r-dlp-warm', rust_warm)]
            for name, action in (modes if repeat % 2 == 0 else list(reversed(modes))):
                record(name, video, lambda: action(video))
    rust.stdin.close(); rust.wait(timeout=20)

summary = {}
for mode in ['yt-dlp-cold','r-dlp-cold','yt-dlp-warm','r-dlp-warm']:
    sample = [r for r in rows if r['mode']==mode]
    good = [r for r in sample if r['ok']]
    summary[mode] = {'success':len(good),'attempts':len(sample),
        'median_resolve_ms':statistics.median([r['resolve_ms'] for r in good]) if good else None,
        'median_total_ms':statistics.median([r['total_ms'] for r in good]) if good else None}
for phase in ['cold','warm']:
    baseline, candidate = summary['yt-dlp-'+phase], summary['r-dlp-'+phase]
    if baseline['median_resolve_ms'] and candidate['median_resolve_ms']:
        reduction = 1 - candidate['median_resolve_ms']/baseline['median_resolve_ms']
        summary[phase+'_reduction_percent'] = reduction*100
        summary[phase+'_passes'] = reduction>=.5 and baseline['success']==baseline['attempts'] and candidate['success']==candidate['attempts']
        comparisons = {}
        for video in VIDEOS:
            samples = [[r['resolve_ms'] for r in rows if r['mode']==name+'-'+phase and r['video']==video and r['ok']] for name in ['yt-dlp','r-dlp']]
            if all(len(sample)==3 for sample in samples):
                medians = [statistics.median(sample) for sample in samples]
                comparisons[video] = {'yt_dlp_ms':medians[0], 'r_dlp_ms':medians[1], 'reduction_percent':(1-medians[1]/medians[0])*100}
        summary[phase+'_per_video'] = comparisons
        summary[phase+'_all_videos_pass'] = len(comparisons)==len(VIDEOS) and all(r['reduction_percent']>=50 for r in comparisons.values())
print(json.dumps({'summary':summary}),flush=True)
