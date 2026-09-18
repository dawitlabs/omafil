"""JSON-only bridge to optional Linux services. Credentials stay in GTK/GVfs."""
import json
import os
import re
import sys
import time
from urllib.parse import urlsplit

import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

ATTRS = 'standard::name,standard::display-name,standard::type,standard::size,standard::is-hidden,time::modified,access::can-read,access::can-write'
REMOTE_SCHEMES = {'sftp', 'smb', 'dav', 'davs', 'ftp', 'ftps', 'mtp', 'gphoto2', 'afc', 'network', 'smb-browse'}


def fail(message):
    raise ValueError(message)


def remote_file(uri, connecting=False):
    if not isinstance(uri, str) or len(uri) > 8192 or any(ord(c) < 32 for c in uri):
        fail('Enter a valid server address.')
    scheme = uri.split(':', 1)[0].lower()
    if scheme not in REMOTE_SCHEMES or not uri.startswith(scheme + '://'):
        fail('Use an sftp://, smb://, dav://, davs://, ftp:// or ftps:// address.')
    authority = uri.split('://', 1)[1].split('/', 1)[0]
    if (connecting and '@' in authority) or ('@' in authority and ':' in authority.rsplit('@', 1)[0]) or '?' in uri or '#' in uri:
        fail('Enter the server address without credentials, query parameters or fragments. Sign in using the connection dialog.')
    if connecting and scheme not in {'sftp', 'smb', 'dav', 'davs', 'ftp', 'ftps'}:
        fail('Choose a discovered device below, or enter a supported server address.')
    if connecting:
        parsed = urlsplit(uri)
        if not parsed.hostname:
            fail('Enter the server hostname.')
        try:
            parsed.port
        except ValueError:
            fail('Enter a valid server port.')
    return Gio.File.new_for_uri(uri)


def location(value):
    if not isinstance(value, dict):
        fail('A location is required.')
    if value.get('kind') == 'local':
        path = value.get('path', '')
        if not isinstance(path, str) or not os.path.isabs(path) or '\0' in path:
            fail('Choose an absolute local folder path.')
        return Gio.File.new_for_path(path)
    if value.get('kind') == 'remote':
        return remote_file(value.get('uri'))
    fail('This location type is unsupported.')


def safe_error(error):
    if isinstance(error, ValueError):
        return str(error)
    if isinstance(error, GLib.Error):
        messages = {
            Gio.IOErrorEnum.PERMISSION_DENIED: 'Access denied. Check permissions or reconnect with the correct account.',
            Gio.IOErrorEnum.NOT_MOUNTED: 'This location is disconnected. Connect to the server or unlock and reconnect the device.',
            Gio.IOErrorEnum.NOT_FOUND: 'This item is no longer available. Refresh the location.',
            Gio.IOErrorEnum.EXISTS: 'An item with this name already exists. Nothing was overwritten.',
            Gio.IOErrorEnum.NOT_SUPPORTED: 'This operation is not supported by the installed provider. Check the required GVfs backend.',
            Gio.IOErrorEnum.CANCELLED: 'Operation cancelled. Completed files were kept; check the destination for incomplete files.',
            Gio.IOErrorEnum.HOST_NOT_FOUND: 'The server could not be found. Check its address and your connection.',
            Gio.IOErrorEnum.CONNECTION_REFUSED: 'The server refused the connection. Check its address and service.',
            Gio.IOErrorEnum.TIMED_OUT: 'The server or device did not respond in time.',
        }
        for code, message in messages.items():
            if error.matches(Gio.io_error_quark(), code):
                return message
    # Provider exceptions can contain credential-bearing URIs; never forward them.
    return 'The Linux service could not complete this request. Check the connection and installed backend, then retry.'


def public_uri(file):
    uri = file.get_uri()
    # GVfs may include the login name in a mount URI. It is needed to distinguish
    # mounted accounts but must not be stored as a bookmark or exposed in errors.
    return uri


def listing(payload):
    folder = location(payload['location'])
    enumerator = folder.enumerate_children(ATTRS, Gio.FileQueryInfoFlags.NOFOLLOW_SYMLINKS, None)
    entries = []
    truncated = False
    visits = 0
    try:
        while True:
            info = enumerator.next_file(None)
            if info is None:
                break
            visits += 1
            if visits > 10000:
                truncated = True
                break
            if info.get_is_hidden() and not payload.get('showHidden', False):
                continue
            if len(entries) == 1000:
                truncated = True
                break
            child = folder.get_child(info.get_name())
            entries.append({'uri': public_uri(child), 'name': info.get_display_name(),
                            'isDirectory': info.get_file_type() == Gio.FileType.DIRECTORY,
                            'isRegular': info.get_file_type() == Gio.FileType.REGULAR,
                            'size': info.get_size(), 'modified': info.get_attribute_uint64('time::modified')})
    finally:
        enumerator.close(None)
    entries.sort(key=lambda item: (not item['isDirectory'], item['name'].casefold()))
    parent = folder.get_parent()
    return {'uri': public_uri(folder), 'parent': public_uri(parent) if parent else None,
            'entries': entries, 'truncated': truncated}


def mounts(_payload):
    monitor = Gio.VolumeMonitor.get()
    found = []
    for mount in monitor.get_mounts():
        root = mount.get_root()
        if root.get_uri_scheme() not in REMOTE_SCHEMES:
            continue
        found.append({'name': mount.get_name(), 'uri': public_uri(root), 'mounted': True,
                      'canUnmount': mount.can_unmount()})
    for volume in monitor.get_volumes():
        root = volume.get_activation_root()
        if volume.get_mount() or not root or root.get_uri_scheme() not in REMOTE_SCHEMES:
            continue
        found.append({'name': volume.get_name(), 'uri': public_uri(root), 'mounted': False,
                      'canUnmount': False})
    return found


def mount_location(payload, operation=None):
    file = remote_file(payload['uri'], connecting=not payload.get('device', False))
    if operation is None:
        gi.require_version('Gtk', '3.0')
        from gi.repository import Gtk
        if not Gtk.init_check()[0]:
            fail('A desktop session is required to connect and show sign-in prompts.')
        operation = Gtk.MountOperation.new(None)
    loop = GLib.MainLoop()
    result = {}

    def done(source, response, finish):
        try:
            finish(response)
            result['uri'] = file.get_uri()
        except GLib.Error as error:
            if error.matches(Gio.io_error_quark(), Gio.IOErrorEnum.ALREADY_MOUNTED):
                result['uri'] = file.get_uri()
            else:
                result['error'] = safe_error(error)
        loop.quit()

    volume = next((v for v in Gio.VolumeMonitor.get().get_volumes()
                   if v.get_activation_root() and v.get_activation_root().equal(file)), None)
    if volume and not volume.get_mount():
        def mounted(source, response, _data):
            done(source, response, source.mount_finish)
            if 'uri' in result and volume.get_mount():
                result['uri'] = volume.get_mount().get_root().get_uri()
        volume.mount(Gio.MountMountFlags.NONE, operation, None, mounted, None)
    else:
        file.mount_enclosing_volume(Gio.MountMountFlags.NONE, operation, None,
                                   lambda source, response, _data: done(source, response, source.mount_enclosing_volume_finish), None)
    loop.run()
    if 'error' in result:
        fail(result['error'])
    return result


def copy_items(payload):
    sources = payload.get('sources', [])
    if not sources or len(sources) > 128:
        fail('Select between 1 and 128 items to copy.')
    destination = location(payload['destination'])
    if destination.query_file_type(Gio.FileQueryInfoFlags.NONE, None) != Gio.FileType.DIRECTORY:
        fail('The destination folder is unavailable.')
    visited = 0
    copied = 0

    def copy_one(source, target, depth=0):
        nonlocal visited
        visited += 1
        if visited > 10000 or depth > 64:
            fail('This copy reached its item or nesting limit. Copy smaller folders separately.')
        info = source.query_info(ATTRS, Gio.FileQueryInfoFlags.NOFOLLOW_SYMLINKS, None)
        kind = info.get_file_type()
        if kind == Gio.FileType.DIRECTORY:
            target.make_directory(None)  # Existing destinations always fail; no merging.
            children = source.enumerate_children(ATTRS, Gio.FileQueryInfoFlags.NOFOLLOW_SYMLINKS, None)
            try:
                while True:
                    child = children.next_file(None)
                    if child is None:
                        break
                    copy_one(source.get_child(child.get_name()), target.get_child(child.get_name()), depth + 1)
            finally:
                children.close(None)
        elif kind == Gio.FileType.REGULAR:
            source.copy(target, Gio.FileCopyFlags.NOFOLLOW_SYMLINKS, None, None, None)
        else:
            fail('Links and special files are not supported by this transfer. Copy regular files and folders.')

    failures = []
    for item in sources:
        source = location(item)
        name = source.get_basename()
        if not name or '/' in name or name in {'.', '..'}:
            fail('This item cannot be copied.')
        target = destination.get_child(name)
        try:
            if target.equal(source) or destination.equal(source) or destination.has_prefix(source):
                fail('A folder cannot be copied into itself.')
            copy_one(source, target)
            copied += 1
        except (GLib.Error, ValueError) as error:
            failures.append({'name': name, 'message': safe_error(error)})
        print(json.dumps({'progress': {'completed': copied, 'total': len(sources)}}), flush=True)
    return {'copied': copied, 'failures': failures}


def indexed_search(payload):
    try:
        gi.require_version('Tsparql', '3.0')
        from gi.repository import Tsparql
    except (ImportError, ValueError):
        fail('Indexed content search requires TinySPARQL and LocalSearch. Filename search is still available.')
    scope = payload.get('path', '')
    if not os.path.isabs(scope) or not os.path.isdir(scope):
        fail('Choose an available local folder to search.')
    scope = os.path.realpath(scope)
    query = payload.get('query', '').strip()
    if not query or len(query) > 500:
        fail('Enter between 1 and 500 characters to search.')
    # Quote words as FTS literals; bind parameters separately from SPARQL syntax.
    words = re.findall(r'\w+', query, re.UNICODE)
    if not words:
        return {'uris': [], 'truncated': False}
    if len(words) > 32:
        fail('Use up to 32 search words at a time.')
    fts = ' AND '.join('"' + word.replace('"', '""') + '"' for word in words[:32])
    try:
        connection = Tsparql.SparqlConnection.bus_new('org.freedesktop.LocalSearch3', None, None)
        statement = connection.query_statement('''
          SELECT DISTINCT ?url WHERE {
            ?document fts:match ~terms ; nie:isStoredAs ?file .
            ?file nie:url ?url .
            FILTER (STRSTARTS(STR(?url), ~scope))
          } ORDER BY ?url LIMIT 501
        ''', None)
        statement.bind_string('terms', fts)
        statement.bind_string('scope', Gio.File.new_for_path(scope).get_uri().rstrip('/') + '/')
        cursor = statement.execute(None)
        uris = []
        examined = 0
        while cursor.next(None):
            examined += 1
            uri = cursor.get_string(0)[0]
            file = Gio.File.new_for_uri(uri)
            path = file.get_path()
            if not path or not os.path.exists(path):
                continue
            if not payload.get('showHidden', False) and any(part.startswith('.') for part in os.path.relpath(path, scope).split(os.sep)):
                continue
            uris.append(uri)
        cursor.close()
        connection.close()
        return {'uris': uris[:500], 'truncated': examined > 500}
    except GLib.Error:
        fail('LocalSearch is unavailable or its index is not ready. Use filename search or check the LocalSearch service.')


def capabilities(_payload):
    try:
        gi.require_version('Tsparql', '3.0')
        indexed = True
    except ValueError:
        indexed = False
    return {'schemes': list(Gio.Vfs.get_default().get_supported_uri_schemes()), 'indexedSearch': indexed}


OPERATIONS = {'capabilities': capabilities, 'mounts': mounts, 'connect': mount_location,
              'list': listing, 'copy': copy_items, 'indexed-search': indexed_search}


def main():
    try:
        operation = sys.argv[1]
        if operation not in OPERATIONS:
            fail('Unsupported Linux service operation.')
        payload = json.load(sys.stdin)
        result = OPERATIONS[operation](payload)
        print(json.dumps({'result': result}), flush=True)
    except Exception as error:
        print(json.dumps({'error': safe_error(error)}), flush=True)


if __name__ == '__main__':
    main()
