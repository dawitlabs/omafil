#!/usr/bin/env python3
"""Exercise real setup/restore in disposable XDG directories.

python3 src/tests/desktop-integration.py [--native]
Build src-tauri/target/debug/omafil with tauri/custom-protocol first.
--native also tests cold D-Bus activation on the current graphical display.
The user's actual default application and configuration are never changed.
"""
import argparse
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
from xml.sax.saxutils import escape

REPO = Path(__file__).resolve().parents[2]


def run(*args, **kwargs):
    result = subprocess.run(args, text=True, capture_output=True, timeout=45, **kwargs)
    if result.returncode:
        raise RuntimeError(f'{args[0]} failed: {result.stdout}\n{result.stderr}')
    return result.stdout.strip()


def dbus(method, *args):
    return run('gdbus', 'call', '--session', '--dest', 'org.freedesktop.DBus',
               '--object-path', '/org/freedesktop/DBus', '--method',
               f'org.freedesktop.DBus.{method}', *args)


def cold_child(binary, root):
    fixture = root / 'Project Files é%#'
    fixture.mkdir()
    file = fixture / '.requested.txt'
    file.write_text('cold activation fixture\n')
    owner_pid = None
    try:
        assert 'false' in dbus('NameHasOwner', 'org.freedesktop.FileManager1')
        run('gdbus', 'call', '--session', '--dest', 'org.freedesktop.FileManager1',
            '--object-path', '/org/freedesktop/FileManager1', '--method',
            'org.freedesktop.FileManager1.ShowItems', json.dumps([file.as_uri()]), '')
        owner = dbus('GetConnectionUnixProcessID', 'org.freedesktop.FileManager1')
        owner_pid = int(re.search(r'uint32 (\d+)', owner).group(1))
        assert Path(f'/proc/{owner_pid}/exe').resolve() == binary.resolve()
        run(str(binary), '--select', '.requested.txt', cwd=fixture)
        assert dbus('GetConnectionUnixProcessID', 'org.freedesktop.FileManager1') == owner
        print('PASS cold D-Bus activation and subsequent launch reuse', flush=True)
    finally:
        if owner_pid is not None:
            os.kill(owner_pid, signal.SIGTERM)


def exercise(binary, native):
    with tempfile.TemporaryDirectory(prefix='omafil-integration-') as temporary:
        root = Path(temporary)
        config, data, bins = root / 'config', root / 'data', root / 'bin'
        config.mkdir()
        (data / 'applications').mkdir(parents=True)
        bins.mkdir()
        (bins / 'omafil').symlink_to(binary)
        (data / 'applications/omafil.desktop').write_text((REPO / 'packaging/omafil.desktop').read_text())
        # A valid prior handler makes this test independent of installed desktop apps.
        (data / 'applications/org.gnome.Nautilus.desktop').write_text('[Desktop Entry]\nType=Application\nName=Previous fixture handler\nExec=/bin/true %U\nMimeType=inode/directory;\n')
        mime = config / 'hyprland-mimeapps.list'
        mime.write_text('[Default Applications]\ninode/directory=org.gnome.Nautilus.desktop;\ntext/plain=editor.desktop;\n')
        service = data / 'dbus-1/services/org.freedesktop.FileManager1.service'
        service.parent.mkdir(parents=True)
        previous_service = '[D-BUS Service]\nName=org.freedesktop.FileManager1\nExec=/usr/bin/nautilus --gapplication-service\n'
        service.write_text(previous_service)
        environment = dict(os.environ, XDG_CONFIG_HOME=str(config), XDG_DATA_HOME=str(data),
                           XDG_STATE_HOME=str(root / 'state'), XDG_CACHE_HOME=str(root / 'cache'),
                           XDG_CONFIG_DIRS=str(root / 'system-config'), XDG_CURRENT_DESKTOP='Hyprland',
                           PATH=f'{bins}:{os.environ["PATH"]}', NO_AT_BRIDGE='1', GIO_USE_VFS='local', GTK_USE_PORTAL='0')
        run(str(binary), '--make-default', env=environment)
        assert run('xdg-mime', 'query', 'default', 'inode/directory', env=environment) == 'omafil.desktop'
        run(str(binary), '--make-default', env=environment)
        assert ' --desktop-service' in service.read_text()
        print('PASS real CLI registration, MIME lookup and repeated setup', flush=True)

        if native:
            # Explicit service directory prevents unrelated session daemons from
            # auto-starting. No readiness probe activates another file manager.
            bus = root / 'session.conf'
            bus.write_text(f'<busconfig><type>session</type><listen>unix:tmpdir=/tmp</listen>'
                           f'<policy context="default"><allow send_destination="*"/><allow receive_sender="*"/>'
                           f'<allow own="*"/></policy><servicedir>{escape(str(service.parent))}</servicedir></busconfig>')
            if subprocess.run(['which', 'hyprctl'], capture_output=True).returncode == 0:
                instances = json.loads(run('hyprctl', 'instances', '-j'))
                if instances:
                    environment['HYPRLAND_INSTANCE_SIGNATURE'] = instances[0]['instance']
            output = run('dbus-run-session', '--config-file', str(bus), '--', sys.executable,
                         str(Path(__file__).resolve()), '--cold-child', str(root), '--binary', str(binary), env=environment)
            print(output, flush=True)

        mime.write_text(mime.read_text().replace('editor.desktop', 'new-editor.desktop'))
        run(str(binary), '--restore-default', env=environment)
        assert run('xdg-mime', 'query', 'default', 'inode/directory', env=environment) == 'org.gnome.Nautilus.desktop'
        assert 'text/plain=new-editor.desktop;' in mime.read_text()
        assert service.read_text() == previous_service
        assert not (config / 'omafil/desktop-integration.json').exists()
        print('PASS restore preserves the prior launcher and later unrelated edits', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, default=REPO / 'src-tauri/target/debug/omafil')
    parser.add_argument('--native', action='store_true')
    parser.add_argument('--cold-child', type=Path)
    args = parser.parse_args()
    if args.cold_child:
        cold_child(args.binary, args.cold_child)
    else:
        exercise(args.binary, args.native)
