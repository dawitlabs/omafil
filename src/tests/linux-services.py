#!/usr/bin/python3
"""Native GIO tests on disposable files. Optional isolated TinySPARQL endpoint."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
BRIDGE = ROOT / 'src-tauri/services/linux_services.py'
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('linux_services', BRIDGE)
services = importlib.util.module_from_spec(spec)
spec.loader.exec_module(services)


def local(path):
    return {'kind': 'local', 'path': str(path)}


class ServicesTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.source = self.root / 'source'
        self.destination = self.root / 'destination'
        self.source.mkdir()
        self.destination.mkdir()

    def tearDown(self):
        self.temp.cleanup()

    def copy(self, source=None, destination=None):
        with contextlib.redirect_stdout(io.StringIO()):
            return services.copy_items({'sources': [local(source or self.source)], 'destination': local(destination or self.destination)})

    def test_remote_addresses_reject_embedded_credentials_and_other_schemes(self):
        for uri in ['sftp://user:secret@host/path', 'smb://user@host/share', 'https://example.com', 'sftp://host/a?secret=secret', 'sftp://host/a#secret', 'sftp://host:invalid/path']:
            with self.assertRaises(ValueError):
                services.remote_file(uri, connecting=True)
        self.assertEqual(services.remote_file('sftp://server/path', connecting=True).get_uri_scheme(), 'sftp')
        self.assertEqual(services.remote_file('mtp://[usb:001,002]/').get_uri_scheme(), 'mtp')
        self.assertEqual(services.remote_file('sftp://alice@server/path').get_uri_scheme(), 'sftp')

    def test_listing_preserves_uris_hidden_policy_and_entry_types(self):
        (self.source / '.hidden').write_text('hidden')
        (self.source / 'ሰላም #%.txt').write_text('hello')
        (self.source / 'folder').mkdir()
        (self.source / 'link').symlink_to('missing')
        result = services.listing({'location': local(self.source)})
        self.assertEqual([e['name'] for e in result['entries']], ['folder', 'link', 'ሰላም #%.txt'])
        self.assertIn('%23%25', result['entries'][-1]['uri'])
        self.assertFalse(result['entries'][1]['isRegular'])
        self.assertEqual(len(services.listing({'location': local(self.source), 'showHidden': True})['entries']), 4)

    def test_large_listing_reports_its_limit(self):
        for i in range(1001): (self.source / str(i)).touch()
        result = services.listing({'location': local(self.source)})
        self.assertEqual(len(result['entries']), 1000)
        self.assertTrue(result['truncated'])

    def test_nested_copy_preserves_bytes_and_refuses_existing_names(self):
        (self.source / 'nested').mkdir()
        data = bytes(range(256)) * 1000
        (self.source / 'nested' / 'binary').write_bytes(data)
        result = self.copy()
        self.assertEqual(result['copied'], 1)
        self.assertEqual((self.destination / 'source/nested/binary').read_bytes(), data)
        (self.destination / 'source/nested/binary').write_bytes(b'keep me')
        result = self.copy()
        self.assertEqual(result['copied'], 0)
        self.assertTrue(result['failures'])
        self.assertEqual((self.destination / 'source/nested/binary').read_bytes(), b'keep me')

    def test_copy_rejects_recursion_and_symlinks(self):
        nested = self.source / 'nested'
        nested.mkdir()
        result = self.copy(destination=nested)
        self.assertEqual(result['copied'], 0)
        self.assertFalse((nested / 'source').exists())
        link = self.source / 'link'
        link.symlink_to(self.root)
        result = self.copy(source=link)
        self.assertEqual(result['copied'], 0)
        self.assertFalse((self.destination / 'link').exists())

    def test_error_messages_do_not_echo_provider_credentials(self):
        error = services.GLib.Error('sftp://user:private@host/path')
        self.assertNotIn('private', services.safe_error(error))

    def test_json_protocol_is_parseable(self):
        result = subprocess.run(['/usr/bin/python3', '-I', str(BRIDGE), 'list'], input=json.dumps({'location': local(self.source)}), text=True, capture_output=True, check=True)
        self.assertEqual(json.loads(result.stdout)['result']['entries'], [])


def endpoint(fixture):
    import gi
    gi.require_version('Tsparql', '3.0')
    from gi.repository import Tsparql, Gio, GLib
    connection = Tsparql.SparqlConnection.new(Tsparql.SparqlConnectionFlags.NONE, None, Tsparql.sparql_get_ontology_nepomuk(), None)
    files = [Path(fixture) / 'scope/report.txt', Path(fixture) / 'outside/report.txt', Path(fixture) / 'scope/.hidden.txt', Path(fixture) / 'scope/missing.txt']
    for number, file in enumerate(files):
        file.parent.mkdir(exist_ok=True)
        if number != 3: file.write_text('quartznectar hidden in document contents')
        uri = file.as_uri()
        update = '''INSERT DATA {
          GRAPH tracker:FileSystem { <%s> a nfo:FileDataObject ; nie:url "%s" . }
          GRAPH tracker:Documents { <urn:test:doc%d> a nfo:TextDocument ; nie:isStoredAs <%s> ; nie:plainTextContent "quartznectar hidden in document contents" . }
        }''' % (uri, uri, number, uri)
        connection.update(update, None)
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    exported = Tsparql.EndpointDBus.new(connection, bus, None, None)
    loop = GLib.MainLoop()
    def acquired(*_args): print('READY', flush=True)
    Gio.bus_own_name_on_connection(bus, 'org.freedesktop.LocalSearch3', Gio.BusNameOwnerFlags.NONE, acquired, None)
    loop.run()


def indexed_contract():
    # Caller must supply a private bus; never register this on the user's bus.
    if not os.environ.get('OMAFIL_PRIVATE_INDEX_TEST'):
        raise RuntimeError('Run with dbus-run-session and OMAFIL_PRIVATE_INDEX_TEST=1')
    with tempfile.TemporaryDirectory() as root:
        process = subprocess.Popen(['/usr/bin/python3', __file__, '--endpoint', root], stdout=subprocess.PIPE, text=True)
        try:
            if process.stdout.readline().strip() != 'READY': raise RuntimeError('Endpoint did not start')
            result = services.indexed_search({'path': str(Path(root) / 'scope'), 'query': 'quartznectar'})
            assert result['uris'] == [(Path(root) / 'scope/report.txt').as_uri()], result
            hidden = services.indexed_search({'path': str(Path(root) / 'scope'), 'query': 'quartznectar', 'showHidden': True})
            assert len(hidden['uris']) == 2, hidden
            injection = services.indexed_search({'path': str(Path(root) / 'scope'), 'query': '\") . ?a ?b ?c #'} )
            assert injection['uris'] == [], injection
            print('PASS native TinySPARQL document-content query, scope, hidden files, stale entries and parameter binding')
        finally:
            process.terminate()
            process.wait(timeout=10)


def ftp_server(root):
    sys.path.insert(0, os.environ['OMAFIL_FTP_MODULES'])
    from pyftpdlib.authorizers import DummyAuthorizer
    from pyftpdlib.handlers import FTPHandler
    from pyftpdlib.servers import FTPServer
    authorizer = DummyAuthorizer()
    authorizer.add_anonymous(root, perm='elradfmwMT')
    handler = FTPHandler
    handler.authorizer = authorizer
    server = FTPServer(('127.0.0.1', 0), handler)
    print(server.socket.getsockname()[1], flush=True)
    server.serve_forever()


def ftp_contract():
    if not os.environ.get('OMAFIL_PRIVATE_INDEX_TEST'):
        raise RuntimeError('Run on a private D-Bus session')
    with tempfile.TemporaryDirectory() as root:
        root = Path(root)
        server_root = root / 'server'
        server_root.mkdir()
        local_root = root / 'local'
        local_root.mkdir()
        (server_root / 'hello ሰላም.txt').write_text('Network round-trip fixture')
        (local_root / 'upload.txt').write_bytes(bytes(range(256)) * 1024)
        process = subprocess.Popen(['/usr/bin/python3', __file__, '--ftp-server', str(server_root)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        try:
            port = int(process.stdout.readline().strip())
            uri = f'ftp://127.0.0.1:{port}/'
            operation = services.Gio.MountOperation.new()
            def authenticate(op, message, user, domain, flags):
                op.set_anonymous(True)
                op.reply(services.Gio.MountOperationResult.HANDLED)
            operation.connect('ask-password', authenticate)
            connected = services.mount_location({'uri': uri}, operation=operation)
            listing = services.listing({'location': {'kind': 'remote', 'uri': connected['uri']}})
            assert listing['entries'][0]['name'] == 'hello ሰላም.txt', listing
            download = services.copy_items({'sources': [{'kind': 'remote', 'uri': listing['entries'][0]['uri']}], 'destination': local(local_root)})
            assert download['copied'] == 1, download
            assert (local_root / 'hello ሰላም.txt').read_bytes() == (server_root / 'hello ሰላም.txt').read_bytes()
            upload = services.copy_items({'sources': [local(local_root / 'upload.txt')], 'destination': {'kind': 'remote', 'uri': uri}})
            assert upload['copied'] == 1, upload
            assert (server_root / 'upload.txt').read_bytes() == (local_root / 'upload.txt').read_bytes()
            again = services.copy_items({'sources': [local(local_root / 'upload.txt')], 'destination': {'kind': 'remote', 'uri': uri}})
            assert again['copied'] == 0 and again['failures'], again
            print('PASS real GVfs FTP connect, listing, download/upload byte equality and no overwrite')
        finally:
            process.terminate()
            process.wait(timeout=10)


def private_contract(mode):
    with tempfile.TemporaryDirectory(prefix='omafil-services-') as directory:
        root = Path(directory)
        service_dir = root / 'services'
        service_dir.mkdir()
        for service in Path('/usr/share/dbus-1/services').glob('org.gtk.vfs.*.service'):
            (service_dir / service.name).symlink_to(service)
        configuration = root / 'bus.conf'
        configuration.write_text(f'<busconfig><type>session</type><listen>unix:tmpdir={directory}</listen><servicedir>{service_dir}</servicedir><policy context="default"><allow send_destination="*"/><allow receive_sender="*"/><allow own="*"/></policy></busconfig>')
        environment = os.environ.copy()
        for variable, name in [('XDG_RUNTIME_DIR', 'runtime'), ('XDG_CACHE_HOME', 'cache'), ('XDG_CONFIG_HOME', 'config'), ('XDG_STATE_HOME', 'state')]:
            path = root / name
            path.mkdir(mode=0o700)
            environment[variable] = str(path)
        environment['OMAFIL_PRIVATE_INDEX_TEST'] = '1'
        environment['GVFS_DISABLE_FUSE'] = '1'
        subprocess.run(['dbus-run-session', '--config-file=' + str(configuration), '--', '/usr/bin/python3', __file__, mode], env=environment, check=True, timeout=90)


if __name__ == '__main__':
    if '--private-indexed' in sys.argv: private_contract('--indexed')
    elif '--private-ftp' in sys.argv: private_contract('--ftp')
    elif '--ftp-server' in sys.argv: ftp_server(sys.argv[-1])
    elif '--ftp' in sys.argv: ftp_contract()
    elif '--endpoint' in sys.argv: endpoint(sys.argv[-1])
    elif '--indexed' in sys.argv: indexed_contract()
    else: unittest.main()
