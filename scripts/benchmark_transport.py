"""Compare persistent HTTP transports on one egress; never log signed URLs."""
import argparse
import hashlib
import json
import os
import platform
import queue
import statistics
import subprocess
import threading
import time
import urllib.request
from contextlib import ExitStack


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary')
    parser.add_argument('--rounds', type=int, default=3)
    args = parser.parse_args()
    if args.rounds < 1:
        parser.error('--rounds must be positive')
    videos = ['dQw4w9WgXcQ', '9bZkp7q19f0', 'M7lc1UVf-VE', 'jNQXAC9IVRw']
    print(json.dumps({'environment': platform.platform(), 'rounds': args.rounds,
                      'binary_sha256': hashlib.sha256(open(args.binary, 'rb').read()).hexdigest(),
                      'definition': 'prewarmed persistent workers; fresh resolution; separate media probe'}), flush=True)
    rows = []
    with ExitStack() as cleanup:
        workers = {}
        for mode in ['negotiated', 'http1']:
            started = time.perf_counter()
            env = dict(os.environ, RDLP_HTTP1_ONLY='1' if mode == 'http1' else '0')
            process = subprocess.Popen([args.binary, 'serve', '--warmup'], env=env,
                                       stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                       stderr=subprocess.DEVNULL, text=True, encoding='utf-8')
            cleanup.callback(stop, process)
            lines = queue.Queue()
            threading.Thread(target=read_lines, args=(process.stdout, lines), daemon=True).start()
            assert json.loads(lines.get(timeout=10)).get('ready'), 'Worker not ready'
            workers[mode] = process, lines
            print(json.dumps({'mode': mode, 'ready_ms': (time.perf_counter()-started)*1000}), flush=True)
        for repeat in range(args.rounds):
            for video in videos:
                for mode in (['negotiated', 'http1'] if repeat % 2 == 0 else ['http1', 'negotiated']):
                    process, lines = workers[mode]
                    started = time.perf_counter()
                    process.stdin.write(json.dumps({'id': video}) + '\n')
                    process.stdin.flush()
                    audio = json.loads(lines.get(timeout=12))
                    elapsed = (time.perf_counter()-started)*1000
                    row = {'mode': mode, 'video': video, 'resolve_ms': elapsed, 'ok': False}
                    if audio.get('id') == video and 'url' in audio:
                        try:
                            request = urllib.request.Request(audio['url'], headers={'Range': 'bytes=0-1023'})
                            with urllib.request.urlopen(request, timeout=15) as response:
                                row['ok'] = response.status == 206 and len(response.read(1024)) == 1024
                        except Exception as error:
                            row['probe_error'] = type(error).__name__
                    rows.append(row)
                    print(json.dumps(row), flush=True)
    passed = all(row['ok'] for row in rows)
    print(json.dumps({'all_passed': passed, 'summary': {
        mode: {'samples': len([r for r in rows if r['mode'] == mode]),
               'median_resolve_ms': statistics.median(r['resolve_ms'] for r in rows if r['mode'] == mode)}
        for mode in workers}}), flush=True)
    return 0 if passed else 1


def read_lines(stream, lines):
    for line in stream:
        lines.put(line)


def stop(process):
    process.stdin.close()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()
    process.stdout.close()


if __name__ == '__main__':
    raise SystemExit(main())
