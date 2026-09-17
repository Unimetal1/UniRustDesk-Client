"""Run the restored client CNG example against local hbbs/hbbr test processes.

Arguments: local non-loopback IPv4 address, public technician .cer file.
Requires PyNaCl, built server debug binaries, and the client debug example.
Uses the existing Windows machine certificate; never exports its private key.
"""
import base64
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time

from nacl.bindings import crypto_sign_keypair

client = Path(__file__).resolve().parents[1]
server = client.parent / 'rustdesk-server'
host = sys.argv[1]
certificate = Path(sys.argv[2]).resolve()
if host.startswith('127.'):
    raise ValueError('Use a LAN address: hbbr reserves loopback for its console')
public_key, secret_key = crypto_sign_keypair()
with tempfile.TemporaryDirectory(prefix='rustdesk-136-') as temporary:
    work = Path(temporary)
    allowed = work / 'technicians'
    allowed.mkdir()
    shutil.copyfile(certificate, allowed / 'technician.cer')
    processes = []
    try:
        env = dict(os.environ, TECHNICIAN_WHITELIST_DIR=str(allowed),
                   DB_URL=str(work / 'db.sqlite3'), RUST_LOG='info')
        for name, port in [('hbbs', 21116), ('hbbr', 21117)]:
            executable = server / 'target/debug' / (name + '.exe')
            with (work / (name + '.log')).open('wb') as log:
                process = subprocess.Popen(
                    [str(executable), '-b', host, '-p', str(port), '-k', base64.b64encode(secret_key).decode()],
                    cwd=work, env=env, stdout=log, stderr=subprocess.STDOUT,
                    creationflags=subprocess.CREATE_NO_WINDOW)
            processes.append(process)
            for attempt in range(100):
                if process.poll() is not None:
                    raise RuntimeError((work / (name + '.log')).read_text())
                try:
                    with socket.create_connection((host, port + 2), timeout=.2):
                        break
                except OSError:
                    time.sleep(.05)
            else:
                raise RuntimeError(f'{name} failed to start')
        test_env = dict(os.environ, RUSTDESK_TEST_HOST=host,
                        RUSTDESK_TEST_PUBLIC_KEY=base64.b64encode(public_key).decode())
        result = subprocess.run([str(client / 'target/debug/examples/cng_compatibility.exe')],
                                env=test_env, cwd=work, capture_output=True, text=True, timeout=90,
                                creationflags=subprocess.CREATE_NO_WINDOW)
        print(result.stdout, end='')
        print(result.stderr, end='', file=sys.stderr)
        result.check_returncode()
    finally:
        for process in processes:
            if process.poll() is None:
                process.terminate()
            process.wait(timeout=10)
