#!/usr/bin/env python3
"""Exercise the real binary against a simulated OpenDeck host. Requires websockets."""
import asyncio, base64, json, os, tempfile
from pathlib import Path
import websockets
ROOT=Path(__file__).resolve().parent.parent
ACTION='se.fredrik.codexusage.usage'
async def main():
    with tempfile.TemporaryDirectory() as tmp:
        state=Path(tmp)/'state';log=Path(tmp)/'log';state.write_text('{}')
        connected=asyncio.get_running_loop().create_future()
        async def connection(ws):
            connected.set_result(ws)
            await ws.wait_closed()
        async with websockets.serve(connection,'127.0.0.1',0) as server:
            port=server.sockets[0].getsockname()[1]
            env={**os.environ,'CODEX_USAGE_CLI':str(ROOT/'tests/fake_codex.py'),'QUOTA_TEST_STATE':str(state),'QUOTA_TEST_LOG':str(log)}
            p=await asyncio.create_subprocess_exec(str(ROOT/'target/debug/opendeck-codex-usage'),'-port',str(port),'-pluginUUID','test','-registerEvent','registerPlugin','-info','{"devices":[]}',env=env)
            try:
                ws=await asyncio.wait_for(connected,5)
                assert json.loads(await ws.recv())['event']=='registerPlugin'
                def event(kind,context='a',window='primary'):
                    return {'event':kind,'action':ACTION,'context':context,'device':'test','payload':{'settings':{'window':window},'controller':'Keypad','coordinates':{'row':0,'column':0},'state':0,'isInMultiAction':False}}
                async def send(kind,context='a',window='primary'): await ws.send(json.dumps(event(kind,context,window)))
                async def image(context,text):
                    async with asyncio.timeout(30):
                        while True:
                            d=json.loads(await ws.recv())
                            if d.get('event')=='setImage' and d.get('context')==context:
                                svg=base64.b64decode(d['payload']['image'].split(',')[1]).decode()
                                if text in svg: return svg
                await send('willAppear');await image('a','23% used')
                await send('willAppear','b','secondary');await image('b','46% used')
                requests=lambda:[json.loads(l) for l in log.read_text().splitlines()]
                assert sum(x['method']=='account/rateLimits/read' for x in requests())==1,'Keys must share a fetch'
                await send('didReceiveSettings','a','secondary');await image('a','46% used')
                await send('didReceiveSettings','a','missing');await image('a','N/A')
                await send('didReceiveSettings');await image('a','23% used')
                await asyncio.sleep(2.1)
                state.write_text('{"error":true}')
                await send('keyUp');svg=await image('a','STALE');assert '23% used' in svg
                await asyncio.sleep(2.1)
                state.write_text('{"account":"changed@example.invalid","error":true}')
                await send('keyUp');svg=await image('a','QUOTA ERROR');assert '23% used' not in svg
                await asyncio.sleep(2.1)
                state.write_text('{"signed_out":true}')
                await send('keyUp');await image('a','SIGN IN')
                await asyncio.sleep(2.1)
                state.write_text('{"percent":31}')
                await send('keyUp');await image('a','31% used')
                await asyncio.sleep(2.1)
                state.write_text('{"disconnect":true}')
                await send('keyUp');await image('a','STALE')
                await asyncio.sleep(2.1)
                state.write_text('{"timeout":true}')
                await send('keyUp')
                # Drain the previous stale image and wait for the timeout status.
                async with asyncio.timeout(30):
                    while True:
                        d=json.loads(await ws.recv())
                        if d.get('event')=='sendToPropertyInspector' and d['payload'].get('status')=='TIMEOUT': break
                await asyncio.sleep(2.1)
                state.write_text('{"percent":37}')
                await send('keyUp');await image('a','37% used')
                pids={x['pid'] for x in requests()}
                assert {x['method'] for x in requests()}=={'initialize','account/read','account/rateLimits/read'}

                await ws.close();assert await asyncio.wait_for(p.wait(),5)==0
                for pid in pids:
                    try: os.kill(pid,0)
                    except ProcessLookupError: continue
                    raise AssertionError(f'Child {pid} survived shutdown')
                print('PASS: shared polling, settings, missing windows, stale data, account changes, sign-in recovery, disconnect/timeout recovery, child cleanup')
            finally:
                if p.returncode is None: p.kill();await p.wait()
asyncio.run(main())
