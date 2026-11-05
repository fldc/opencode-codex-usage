#!/usr/bin/env python3
"""Validate the distributable and ensure a release tag matches its version."""
import argparse
import json
from pathlib import Path
import stat
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
PLUGIN = 'se.fredrik.codexusage.sdPlugin'


def check(tag=''):
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    manifest = json.loads((ROOT / 'assets/manifest.json').read_text())
    if manifest['Version'] != version:
        raise ValueError('Cargo.toml and manifest.json versions must match')
    if tag and tag != f'v{version}':
        raise ValueError(f'Tag {tag!r} must match v{version}')
    with zipfile.ZipFile(ROOT / 'dist/codex-usage-linux-x86_64.zip') as archive:
        if archive.testzip() is not None:
            raise ValueError('ZIP integrity check failed')
        required = {'manifest.json', 'pi.html', 'icon.svg', 'LICENSE', 'opendeck-codex-usage'}
        files = {info.filename for info in archive.infolist() if not info.is_dir()}
        if files != {f'{PLUGIN}/{name}' for name in required}:
            raise ValueError('Package contains missing or unexpected files')
        bundled = json.loads(archive.read(f'{PLUGIN}/manifest.json'))
        if bundled != manifest:
            raise ValueError('Bundled manifest does not match the source')
        binary = archive.getinfo(f'{PLUGIN}/opendeck-codex-usage')
        if not (binary.external_attr >> 16) & stat.S_IXUSR:
            raise ValueError('Plugin binary is not executable')
        for name in ('pi.html', 'icon.svg'):
            if archive.read(f'{PLUGIN}/{name}') != (ROOT / 'assets' / name).read_bytes():
                raise ValueError(f'Bundled {name} does not match the source')
    print(f'Validated Linux package v{version}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag', default='')
    check(parser.parse_args().tag)
