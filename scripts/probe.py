#!/usr/bin/env python3
"""Read quota through Codex; print only quota data, never credentials or identity."""
import json, subprocess, selectors, time, os
p = subprocess.Popen(['codex','app-server','--listen','stdio://'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
s = selectors.DefaultSelector(); s.register(p.stdout, selectors.EVENT_READ)
buffer = b''
def send(v):
    p.stdin.write((json.dumps(v)+'\n').encode()); p.stdin.flush()
def rpc(i, method, params=None):
    global buffer
    send(dict(id=i,method=method,params=params or {}))
    deadline=time.monotonic()+30
    while time.monotonic()<deadline:
        while b'\n' not in buffer:
            if not s.select(max(0, deadline-time.monotonic())): raise TimeoutError(method)
            chunk=os.read(p.stdout.fileno(),65536)
            if not chunk: raise RuntimeError('Codex exited')
            buffer+=chunk
        line,buffer=buffer.split(b'\n',1)
        v=json.loads(line)
        if v.get('id')==i:
            if 'error' in v: raise RuntimeError('RPC failed: '+method)
            return v['result']
    raise TimeoutError(method)
try:
    rpc(1,'initialize',{'clientInfo':{'name':'opendeck_codex_usage','version':'0.1.0'}})
    send({'method':'initialized'})
    a=rpc(2,'account/read',{'refreshToken':False}).get('account')
    print('Account type:', a.get('type') if a else 'signed out')
    r=rpc(3,'account/rateLimits/read')
    # Whitelist only quota fields; omit all account identity and credit details.
    def bucket(b):
        return {k:b.get(k) for k in ['limitId','primary','secondary']}
    print(json.dumps({'rateLimits':bucket(r['rateLimits']) if r.get('rateLimits') else None,'rateLimitsByLimitId':{k:bucket(v) for k,v in (r.get('rateLimitsByLimitId') or {}).items()}},indent=2))
finally:
    p.terminate()
    try: p.wait(timeout=3)
    except subprocess.TimeoutExpired: p.kill(); p.wait()
