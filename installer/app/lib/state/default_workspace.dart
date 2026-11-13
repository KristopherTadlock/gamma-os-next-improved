import 'dart:io';

import 'package:path/path.dart' as p;

/// Where the installer keeps downloads and builds when the user hasn't
/// chosen a folder: next to the app, or the app data folder when the app's
/// own folder is read-only.
///
/// Pure: the executable path, OS, environment and the writability test are
/// passed in, so tests can cover every platform. [findDefaultWorkspace]
/// supplies the real ones.
String defaultWorkspaceFor({
  required String executable,
  required String os,
  required Map<String, String> env,
  required bool Function(String dir) isWritable,
}) {
  final ctx = os == 'windows' ? p.windows : p.posix;
  final here = appFolder(executable, os);
  if (here.isNotEmpty &&
      (os != 'macos' || macFolderUsable(here, env)) &&
      isWritable(here)) {
    return here;
  }
  return appDataFolder(os, env, ctx);
}

/// On macOS, the app's folder is used only when it's a folder of the
/// user's own inside their home, such as Downloads or Desktop: never
/// `/Applications`, `/System`, `/Library`, a mounted disk image, a
/// translocated copy, `~/Applications` or `~/Library` (an administrator can
/// write to `/Applications`, but downloads don't belong there).
bool macFolderUsable(String dir, Map<String, String> env) {
  final home = p.posix.normalize(env['HOME'] ?? '');
  if (home.isEmpty || home == '/' || !home.startsWith('/')) return false;
  final d = p.posix.normalize(dir);
  if (!p.posix.isWithin(home, d)) return false;
  // macOS folder names are case-insensitive by default.
  final top = p.posix.split(p.posix.relative(d, from: home)).first;
  return !{'applications', 'library'}.contains(top.toLowerCase());
}

/// The folder the app is in: the executable's folder on Windows and Linux;
/// on macOS the folder that contains `Thimsy Installer.app` (never inside
/// the bundle).
String appFolder(String executable, String os) {
  final ctx = os == 'windows' ? p.windows : p.posix;
  final dir = ctx.dirname(executable);
  if (os != 'macos') return dir;
  // …/Thimsy Installer.app/Contents/MacOS/Thimsy Installer
  var d = dir;
  while (true) {
    if (ctx.basename(d).toLowerCase().endsWith('.app')) return ctx.dirname(d);
    final parent = ctx.dirname(d);
    if (parent == d) return dir; // not in a bundle (unusual): use its folder
    d = parent;
  }
}

/// The per-user app data folder for Thimsy Installer.
String appDataFolder(String os, Map<String, String> env, [p.Context? ctx]) {
  ctx ??= os == 'windows' ? p.windows : p.posix;
  String? nonEmpty(String? v) => (v == null || v.isEmpty) ? null : v;
  final home = nonEmpty(env['HOME']) ?? nonEmpty(env['USERPROFILE']) ?? '';
  switch (os) {
    case 'windows':
      final local =
          nonEmpty(env['LOCALAPPDATA']) ??
          ctx.join(nonEmpty(env['USERPROFILE']) ?? home, 'AppData', 'Local');
      return ctx.join(local, 'Thimsy Installer');
    case 'macos':
      return ctx.join(
        home,
        'Library',
        'Application Support',
        'Thimsy Installer',
      );
    default:
      final data =
          nonEmpty(env['XDG_DATA_HOME']) ?? ctx.join(home, '.local', 'share');
      return ctx.join(data, 'thimsy-installer');
  }
}

/// True when a file can be created (and removed) in [dir].
bool canWriteTo(String dir) {
  try {
    if (!Directory(dir).existsSync()) return false;
    final probe = File(
      p.join(
        dir,
        '.thimsy-write-test-$pid-${DateTime.now().microsecondsSinceEpoch}',
      ),
    );
    probe.writeAsStringSync('');
    probe.deleteSync();
    return true;
  } catch (_) {
    return false;
  }
}

/// The default workspace for this computer, creating the app data folder
/// when that's the one used. Empty when there's no sensible default (the
/// user then picks a folder, as before).
String findDefaultWorkspace() {
  final os = Platform.operatingSystem;
  if (os != 'windows' && os != 'macos' && os != 'linux') return '';
  try {
    final ws = defaultWorkspaceFor(
      executable: Platform.resolvedExecutable,
      os: os,
      env: Platform.environment,
      isWritable: canWriteTo,
    );
    final dir = Directory(ws);
    if (!dir.existsSync()) dir.createSync(recursive: true);
    return ws;
  } catch (_) {
    return '';
  }
}
