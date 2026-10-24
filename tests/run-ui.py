import os, pathlib, subprocess, tempfile, urllib.request
root = pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='atelier-test-') as directory:
    env = dict(os.environ, LOCALAPPDATA=directory, ATELIER_HEADLESS='1')
    server = subprocess.Popen([str(root/'target/debug/atelier-roadmap')], env=env, stdout=subprocess.PIPE, text=True)
    url = server.stdout.readline().strip()
    try:
        result = subprocess.run(['node', str(root/'tests/ui.cjs')], env=dict(os.environ, ATELIER_TEST_URL=url))
        raise SystemExit(result.returncode)
    finally:
        try:
            urllib.request.urlopen(urllib.request.Request(url+'quit',method='POST'),timeout=2).read()
        except Exception:
            pass
        server.terminate()
        server.wait()
