#!/usr/bin/env python3
"""Protocol fixture with controllable accounts, quota failures, and request counts."""
import json, os, sys, time
from pathlib import Path
state_path=Path(os.environ['QUOTA_TEST_STATE'])
for line in sys.stdin:
    v=json.loads(line)
    if 'id' not in v: continue
    s=json.loads(state_path.read_text())
    method=v['method']
    with open(os.environ['QUOTA_TEST_LOG'],'a') as f:
        f.write(json.dumps({'method':method,'pid':os.getpid()})+'\n')
    if method=='initialize': result={}
    elif method=='account/read': result={'account':None if s.get('signed_out') else {'type':'chatgpt','email':s.get('account','fixture@example.invalid')}}
    elif method=='account/rateLimits/read':
        if s.get('disconnect'): sys.exit(0)
        if s.get('timeout'): time.sleep(60)
        if s.get('error'):
            print(json.dumps({'id':v['id'],'error':{'code':-1,'message':'fixture failure'}}),flush=True)
            continue
        result={'rateLimitsByLimitId':{'codex':{'primary':{'usedPercent':s.get('percent',23),'windowDurationMins':300,'resetsAt':2000000000},'secondary':{'usedPercent':46,'windowDurationMins':10080,'resetsAt':2000100000}}}}
    else: raise RuntimeError('Unexpected method: '+method)
    print(json.dumps({'method':'account/rateLimits/updated','params':{}}),flush=True)
    print(json.dumps({'id':99999,'result':{}}),flush=True)
    print(json.dumps({'id':v['id'],'result':result}),flush=True)
