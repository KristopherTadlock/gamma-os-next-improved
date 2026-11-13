import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:path/path.dart' as p;
import 'package:thimsy_installer/engine/fake_engine.dart';
import 'package:thimsy_installer/l10n/app_localizations.dart';
import 'package:thimsy_installer/main.dart';
import 'package:thimsy_installer/state/app_state.dart';
import 'package:thimsy_installer/state/default_workspace.dart';
import 'package:thimsy_installer/widgets/common.dart';

Finder key(String k) => find.byKey(ValueKey(k));

Future<void> tap(WidgetTester tester, Finder f) async {
  await tester.ensureVisible(f);
  await tester.pumpAndSettle();
  await tester.tap(f);
  await tester.pumpAndSettle();
}

/// Starts the app with [workspace] and walks to the build step.
Future<(AppState, FakeEngine)> toBuildStep(
  WidgetTester tester, {
  String workspace = '',
}) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final engine = FakeEngine(step: Duration.zero);
  final state = AppState(engine, onAndroid: false, workspace: workspace);
  await tester.pumpWidget(ThimsyInstallerApp(state: state));
  await tester.pumpAndSettle();
  await tap(tester, key('device-anbernic-rg-cube'));
  await tap(tester, key('next'));
  await tap(tester, key('next'));
  await tap(tester, key('journey-first_time'));
  await tap(tester, key('next'));
  expect(find.text('Download and build'), findsOneWidget);
  return (state, engine);
}

String fieldText(WidgetTester tester, String k) =>
    tester.widget<TextField>(key(k)).controller!.text;

bool isSelected(WidgetTester tester, String k) {
  final tile = tester.widget<RadioListTile<ImageSource>>(key(k));
  final group = tester.widget<RadioGroup<ImageSource>>(
    find.ancestor(of: key(k), matching: find.byType(RadioGroup<ImageSource>)),
  );
  return group.groupValue == tile.value;
}

void main() {
  group('default workspace', () {
    bool yes(String _) => true;
    bool no(String _) => false;

    test('Linux and Windows: the folder the executable is in', () {
      expect(
        defaultWorkspaceFor(
          executable: '/home/me/Thimsy/thimsy_installer',
          os: 'linux',
          env: const {'HOME': '/home/me'},
          isWritable: yes,
        ),
        '/home/me/Thimsy',
      );
      expect(
        defaultWorkspaceFor(
          executable: r'D:\Tools\Thimsy\thimsy_installer.exe',
          os: 'windows',
          env: const {},
          isWritable: yes,
        ),
        r'D:\Tools\Thimsy',
      );
    });

    test('macOS: the folder that contains the .app, never inside it', () {
      final tried = <String>[];
      final ws = defaultWorkspaceFor(
        executable:
            '/Users/me/Apps/Thimsy Installer.app/Contents/MacOS/'
            'Thimsy Installer',
        os: 'macos',
        env: const {'HOME': '/Users/me'},
        isWritable: (d) {
          tried.add(d);
          return true;
        },
      );
      expect(ws, '/Users/me/Apps');
      expect(tried, ['/Users/me/Apps']);
    });

    test('macOS: only a folder of the user\'s own, never Applications', () {
      const env = {'HOME': '/Users/me'};
      const data = '/Users/me/Library/Application Support/Thimsy Installer';
      String ws(String appDir) => defaultWorkspaceFor(
        executable:
            '$appDir/Thimsy Installer.app/Contents/MacOS/'
            'Thimsy Installer',
        os: 'macos',
        env: env,
        // An administrator can write to /Applications: still not used.
        isWritable: (_) => true,
      );
      expect(ws('/Applications'), data);
      expect(ws('/Users/me/Applications'), data);
      expect(ws('/Users/me/applications/Games'), data);
      expect(ws('/Users/me/Library/Caches'), data);
      expect(ws('/System/Applications'), data);
      expect(ws('/Library/Thimsy'), data);
      expect(ws('/Volumes/Thimsy Installer'), data);
      expect(ws('/Users/other/Downloads'), data);
      expect(ws('/Users/me'), data);
      expect(ws('/Users/me/Downloads'), '/Users/me/Downloads');
      expect(ws('/Users/me/Desktop/Thimsy'), '/Users/me/Desktop/Thimsy');
      // Without a home folder there's nothing to compare with.
      expect(
        defaultWorkspaceFor(
          executable:
              '/Users/me/Downloads/Thimsy Installer.app/Contents/'
              'MacOS/Thimsy Installer',
          os: 'macos',
          env: const {},
          isWritable: (_) => true,
        ),
        isNot('/Users/me/Downloads'),
      );
      // Linux and Windows aren't affected.
      expect(
        defaultWorkspaceFor(
          executable: '/opt/thimsy/thimsy_installer',
          os: 'linux',
          env: const {'HOME': '/home/me'},
          isWritable: (_) => true,
        ),
        '/opt/thimsy',
      );
    });

    test('read-only app folder: the app data folder instead', () {
      expect(
        defaultWorkspaceFor(
          executable: r'C:\Program Files\Thimsy\thimsy_installer.exe',
          os: 'windows',
          env: const {'LOCALAPPDATA': r'C:\Users\me\AppData\Local'},
          isWritable: no,
        ),
        r'C:\Users\me\AppData\Local\Thimsy Installer',
      );
      expect(
        defaultWorkspaceFor(
          executable: r'C:\Program Files\Thimsy\thimsy_installer.exe',
          os: 'windows',
          env: const {'USERPROFILE': r'C:\Users\me'},
          isWritable: no,
        ),
        r'C:\Users\me\AppData\Local\Thimsy Installer',
      );
      // A translocated (read-only) macOS app.
      expect(
        defaultWorkspaceFor(
          executable:
              '/private/var/folders/x/AppTranslocation/1234/d/'
              'Thimsy Installer.app/Contents/MacOS/Thimsy Installer',
          os: 'macos',
          env: const {'HOME': '/Users/me'},
          isWritable: no,
        ),
        '/Users/me/Library/Application Support/Thimsy Installer',
      );
      expect(
        defaultWorkspaceFor(
          executable: '/app/bin/thimsy_installer',
          os: 'linux',
          env: const {
            'HOME': '/home/me',
            'XDG_DATA_HOME': '/home/me/.var/app/thimsy/data',
          },
          isWritable: no,
        ),
        '/home/me/.var/app/thimsy/data/thimsy-installer',
      );
      expect(
        defaultWorkspaceFor(
          executable: '/opt/thimsy/thimsy_installer',
          os: 'linux',
          env: const {'HOME': '/home/me'},
          isWritable: no,
        ),
        '/home/me/.local/share/thimsy-installer',
      );
    });

    test('writability is tested with a temporary file', () {
      final dir = Directory.systemTemp.createTempSync('thimsy-ws-');
      addTearDown(() => dir.deleteSync(recursive: true));
      expect(canWriteTo(dir.path), isTrue);
      expect(dir.listSync(), isEmpty, reason: 'the probe file is removed');
      expect(canWriteTo(p.join(dir.path, 'missing')), isFalse);
      if (!Platform.isWindows) {
        final ro = Directory(p.join(dir.path, 'ro'))..createSync();
        Process.runSync('chmod', ['555', ro.path]);
        addTearDown(() => Process.runSync('chmod', ['755', ro.path]));
        // Root can write anywhere; only check as a normal user.
        final root = Process.runSync('id', ['-u']).stdout.toString().trim();
        if (root != '0') expect(canWriteTo(ro.path), isFalse);
      }
    });
  });

  group('build step', () {
    final savedDir = pickDirectory;
    final savedFiles = pickPackFiles;
    tearDown(() {
      pickDirectory = savedDir;
      pickPackFiles = savedFiles;
    });

    testWidgets('the default workspace needs no action', (tester) async {
      final ws = defaultWorkspaceFor(
        executable: '/home/me/Thimsy/thimsy_installer',
        os: 'linux',
        env: const {'HOME': '/home/me'},
        isWritable: (_) => true,
      );
      final (_, engine) = await toBuildStep(tester, workspace: ws);
      expect(fieldText(tester, 'workspace-field'), '/home/me/Thimsy');
      // The space check ran for it on entering the step.
      expect(engine.calls, contains('space /home/me/Thimsy'));
      expect(key('space-summary'), findsOneWidget);
      await tap(tester, key('start-build'));
      expect(key('build-complete'), findsOneWidget);
    });

    testWidgets('Change folder… picks another workspace and rechecks space', (
      tester,
    ) async {
      pickDirectory = () async => '/mnt/big';
      final (state, engine) = await toBuildStep(
        tester,
        workspace: '/home/me/Thimsy',
      );
      expect(find.text('Change folder…'), findsOneWidget);
      await tap(tester, key('change-folder'));
      await tester.pump(const Duration(milliseconds: 500));
      await tester.pumpAndSettle();
      expect(state.workspace, '/mnt/big');
      expect(fieldText(tester, 'workspace-field'), '/mnt/big');
      expect(engine.calls, contains('space /mnt/big'));
    });

    testWidgets('downloading is the default and passes no --import', (
      tester,
    ) async {
      final (state, engine) = await toBuildStep(tester, workspace: '/ws');
      expect(find.text('Download automatically'), findsOneWidget);
      expect(find.text('Use a file I already downloaded…'), findsOneWidget);
      expect(isSelected(tester, 'source-download'), isTrue);
      expect(state.imageSource, ImageSource.download);
      await tap(tester, key('start-build'));
      expect(engine.lastImports, isEmpty);
    });

    testWidgets('a downloaded pack: each chosen part is passed as --import', (
      tester,
    ) async {
      AppLocalizations? pickerLabels;
      pickPackFiles = (l) async {
        pickerLabels = l;
        return ['/dl/rg-cube.tar.002', '/dl/rg-cube.tar.001'];
      };
      final (state, engine) = await toBuildStep(tester, workspace: '/ws');
      await tap(tester, key('source-files'));
      expect(pickerLabels?.packFileType, 'Thimsy device pack');
      expect(isSelected(tester, 'source-files'), isTrue);
      expect(find.text('rg-cube.tar.001'), findsOneWidget);
      expect(find.text('rg-cube.tar.002'), findsOneWidget);
      // Parts are kept in order.
      expect(state.importFiles, ['/dl/rg-cube.tar.001', '/dl/rg-cube.tar.002']);
      await tap(tester, key('start-build'));
      expect(engine.lastImports, [
        '/dl/rg-cube.tar.001',
        '/dl/rg-cube.tar.002',
      ]);

      // Choosing again replaces the files; a single .tar is fine too.
      pickPackFiles = (_) async => ['/dl/rg-cube.tar'];
      await tap(tester, key('choose-files'));
      expect(find.text('rg-cube.tar'), findsOneWidget);
      expect(find.text('rg-cube.tar.001'), findsNothing);
      await tap(tester, key('start-build'));
      expect(engine.lastImports, ['/dl/rg-cube.tar']);

      // Back to downloading: no --import, but the files are remembered.
      await tap(tester, key('source-download'));
      expect(key('chosen-files'), findsNothing);
      await tap(tester, key('start-build'));
      expect(engine.lastImports, isEmpty);
      await tap(tester, key('source-files'));
      expect(find.text('rg-cube.tar'), findsOneWidget);
    });

    testWidgets('cancelling the file picker keeps downloading', (tester) async {
      pickPackFiles = (_) async => const [];
      final (state, _) = await toBuildStep(tester, workspace: '/ws');
      await tap(tester, key('source-files'));
      expect(state.imageSource, ImageSource.download);
      expect(isSelected(tester, 'source-download'), isTrue);
      expect(key('chosen-files'), findsNothing);
      expect(
        tester.widget<FilledButton>(key('start-build')).onPressed,
        isNotNull,
      );
    });
  });
}
