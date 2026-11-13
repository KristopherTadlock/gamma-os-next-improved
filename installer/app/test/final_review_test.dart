// Final review round: update mode, translated error titles, failed driver
// installs and cleans, download mode again, Linux device access, picked
// files in the space check, freeing space before a build, deleting
// backups, the Review partitions row and "Copy details".
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:thimsy_installer/engine/engine.dart';
import 'package:thimsy_installer/engine/fake_engine.dart';
import 'package:thimsy_installer/l10n/app_localizations.dart';
import 'package:thimsy_installer/main.dart';
import 'package:thimsy_installer/screens/review_screen.dart';
import 'package:thimsy_installer/state/app_state.dart';
import 'package:thimsy_installer/widgets/common.dart';

Finder key(String k) => find.byKey(ValueKey(k));

const cube = 'anbernic-rg-cube';
final en = lookupAppLocalizations(const Locale('en'));

class MemoryStore implements LanguageStore {
  String? value;
  MemoryStore([this.value]);
  @override
  String? load() => value;
  @override
  Future<void> save(String? code) async => value = code;
}

Future<AppState> start(
  WidgetTester tester,
  FakeEngine engine, {
  String? language,
  String workspace = '',
}) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final state = AppState(
    engine,
    onAndroid: false,
    workspace: workspace,
    languageStore: language == null ? null : MemoryStore(language),
  );
  await tester.pumpWidget(ThimsyInstallerApp(state: state));
  await tester.pumpAndSettle();
  return state;
}

Future<void> tap(WidgetTester tester, Finder f) async {
  await tester.ensureVisible(f);
  await tester.pumpAndSettle();
  await tester.tap(f);
  await tester.pumpAndSettle();
}

Future<void> next(WidgetTester tester) => tap(tester, key('next'));

/// Device → Profile → Journey → Build step (not built yet).
Future<void> toBuildStep(
  WidgetTester tester, {
  String journey = 'first_time',
  String workspace = '/ws',
}) async {
  await tap(tester, key('device-$cube'));
  await next(tester);
  await next(tester);
  await tap(tester, key('journey-$journey'));
  await next(tester);
  await tester.enterText(key('workspace-field'), workspace);
  await tester.pumpAndSettle();
}

/// … → Build → Backup → (Unlock) → Connect.
Future<void> toConnect(
  WidgetTester tester, {
  String journey = 'first_time',
}) async {
  await toBuildStep(tester, journey: journey);
  await tap(tester, key('start-build'));
  expect(key('build-complete'), findsOneWidget);
  await next(tester);
  await tap(tester, key('backed-up'));
  await next(tester);
  if (journey == 'first_time') {
    await tap(tester, key('unlock-confirm'));
    await next(tester);
  }
  expect(find.text(en.connectTitle), findsOneWidget);
}

Future<void> toReview(
  WidgetTester tester, {
  String journey = 'first_time',
}) async {
  await toConnect(tester, journey: journey);
  expect(key('device-connected'), findsOneWidget);
  await next(tester);
  expect(key('install-button'), findsOneWidget);
}

Future<void> install(WidgetTester tester) async {
  await tap(tester, key('install-button'));
  await tap(tester, key('confirm-install-yes'));
}

List<String> flashes(FakeEngine e) =>
    e.calls.where((c) => c.startsWith('flash')).toList();

FlashMode selectedMode(WidgetTester tester) => tester
    .widget<RadioGroup<FlashMode>>(find.byType(RadioGroup<FlashMode>))
    .groupValue!;

String rowText(WidgetTester tester) => tester
    .widget<SelectableText>(
      find.descendant(
        of: key('row-partitions'),
        matching: find.byType(SelectableText),
      ),
    )
    .data!;

void main() {
  group('C-1 update mode', () {
    test(
      'the returning journey starts in full mode; update is offered',
      () async {
        final engine = FakeEngine(step: Duration.zero);
        final s = AppState(engine, onAndroid: false, workspace: '/ws');
        await s.loadCatalog();
        await s.selectDevice(cube);
        s.deviceNext();
        await s.loadPlans();
        s.selectJourney('returning');
        expect(s.allowedModes, containsAll([FlashMode.update, FlashMode.full]));
        expect(s.flashMode, FlashMode.full);
        s.setFlashMode(FlashMode.update);
        expect(s.flashMode, FlashMode.update);
        // Picking the journey again (another one, then back) resets to full.
        s.selectJourney('first_time');
        s.selectJourney('returning');
        expect(s.flashMode, FlashMode.full);
      },
    );

    testWidgets('full_install_needed: explained, Switch to full goes back to '
        'Review and needs a new confirmation', (tester) async {
      final engine = FakeEngine(step: Duration.zero)
        ..flashErrors.add(
          const ErrorEvent(
            code: 'full_install_needed',
            message: 'device runs GammaOS 1.0, update needs a 1.2 base',
            hint: 'Install with a full install instead.',
          ),
        );
      final s = await start(tester, engine);
      await toReview(tester, journey: 'returning');
      await tap(tester, key('mode-update'));
      await install(tester);
      expect(flashes(engine), ['flash update']);

      expect(find.text(en.errTitleFullInstallNeeded), findsOneWidget);
      expect(
        find.textContaining('Install with a full install instead.'),
        findsOneWidget,
      );
      expect(find.textContaining(en.fullInstallNeededBody), findsOneWidget);
      // Repeating the update would fail again: no Run again.
      expect(key('run-again'), findsNothing);

      await tap(tester, key('switch-to-full'));
      expect(s.step, WizardStep.review);
      expect(s.flashMode, FlashMode.full);
      expect(selectedMode(tester), FlashMode.full);
      expect(find.text(en.eraseWarning), findsOneWidget);
      expect(s.canRunAgain, isFalse);
      // Nothing started without the user confirming the full install.
      expect(flashes(engine), ['flash update']);
      await tap(tester, key('install-button'));
      expect(key('confirm-install'), findsOneWidget);
      expect(find.text(en.confirmFullBody('Anbernic RG Cube')), findsOneWidget);
      await tap(tester, key('confirm-install-yes'));
      expect(flashes(engine), ['flash update', 'flash full']);
      expect(key('install-complete'), findsOneWidget);
    });
  });

  group('C-2 error titles', () {
    testWidgets('engine errors get a translated title; the hint is the body; '
        'the English message is under Details', (tester) async {
      final engine = FakeEngine(step: Duration.zero)
        ..flashErrors.add(
          const ErrorEvent(
            code: 'wrong_device',
            message: 'partition super: device 4.0 GiB, firmware 6.0 GiB',
            hint: 'Ein anderer Hinweis vom Engine.',
          ),
        );
      await start(tester, engine, language: 'de');
      final de = lookupAppLocalizations(const Locale('de'));
      await toBuildStep(tester);
      await tap(tester, key('start-build'));
      await next(tester);
      await tap(tester, key('backed-up'));
      await next(tester);
      await tap(tester, key('unlock-confirm'));
      await next(tester);
      await next(tester);
      await install(tester);

      expect(find.text(de.errTitleWrongDevice), findsOneWidget);
      expect(find.text('Ein anderer Hinweis vom Engine.'), findsOneWidget);
      // English stays out of sight until Details is opened.
      expect(find.textContaining('partition super'), findsNothing);
      await tap(
        tester,
        find.descendant(of: key('run-error'), matching: key('error-details')),
      );
      expect(find.textContaining('partition super'), findsOneWidget);
    });

    test('an unknown code gets a generic title, never the English message', () {
      const e = ErrorEvent(code: 'brand_new', message: 'English text');
      expect(errorTitleFor(en, 'brand_new'), isNull);
      expect(errorTitle(en, e), en.errTitleUnknown);
      expect(describeError(en, const EngineException(e)), en.errTitleUnknown);
    });
  });

  group('C-3 failed driver installs and cleans are shown', () {
    testWidgets('a failed driver install keeps the notice, with the hint', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero, host: 'windows')
        ..driversFailure = const ErrorEvent(
          code: 'tool_failed',
          message: 'wdi-simple exited with 5',
          hint: 'Accept the Windows prompt to install the driver.',
        );
      final s = await start(tester, engine);
      await toConnect(tester);
      expect(key('driver-notice'), findsOneWidget);
      expect(find.text(en.driverNeeded), findsOneWidget);
      await tap(tester, key('install-driver'));
      expect(engine.calls, contains('drivers $cube install'));
      expect(key('driver-notice'), findsOneWidget);
      expect(
        find.textContaining('Accept the Windows prompt to install the driver.'),
        findsOneWidget,
      );
      expect(s.driverError?.code, 'tool_failed');
      expect(s.driverStatus, 'missing');
      // Installing again (accepted this time) clears it.
      engine.driversFailure = null;
      await tap(tester, key('install-driver'));
      expect(key('driver-notice'), findsNothing);
      expect(s.driverError, isNull);
    });

    testWidgets('a failed clean shows an error, not "Freed"', (tester) async {
      final engine = FakeEngine(step: Duration.zero)
        ..cleanFailure = const ErrorEvent(
          code: 'internal',
          message: 'Permission denied (os error 13)',
          hint: 'Close programs using the workspace folder.',
        );
      final s = await start(tester, engine);
      await toReview(tester);
      await install(tester);
      await next(tester);
      await next(tester);
      expect(key('free-up-space'), findsOneWidget);
      await tap(tester, key('run-clean'));
      expect(key('freed'), findsNothing);
      expect(find.textContaining('Freed'), findsNothing);
      expect(key('clean-error'), findsOneWidget);
      expect(find.text(en.cleanFailed), findsOneWidget);
      expect(
        find.textContaining('Close programs using the workspace folder.'),
        findsOneWidget,
      );
      expect(s.freedBytes, isNull);
    });
  });

  group('C-4 device not found', () {
    testWidgets('shows the download-mode steps again and Run again repeats '
        'the confirmed install', (tester) async {
      final engine = FakeEngine(step: Duration.zero)
        ..flashErrors.add(
          const ErrorEvent(
            code: 'device_not_found',
            message: 'no device in download mode after 300 s',
            hint: 'Hold Vol− while plugging in the cable.',
          ),
        );
      await start(tester, engine);
      await toReview(tester);
      await install(tester);
      expect(find.text(en.errTitleDeviceNotFound), findsOneWidget);
      expect(key('download-mode-again'), findsOneWidget);
      // The plan's download-mode guide, again.
      expect(find.text('Enter download mode'), findsOneWidget);
      await tap(tester, key('run-again'));
      expect(flashes(engine), ['flash full', 'flash full']);
      expect(key('install-complete'), findsOneWidget);
      expect(key('download-mode-again'), findsNothing);
    });
  });

  group('C-5 device access on Linux', () {
    testWidgets('Linux: "Allow access to the device" installs the rule', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero, linuxRuleMissing: true);
      await start(tester, engine);
      await toConnect(tester);
      expect(engine.calls, contains('drivers $cube'));
      expect(key('driver-notice'), findsOneWidget);
      expect(find.text(en.driverNeededLinux), findsOneWidget);
      expect(find.textContaining(en.driverLinuxBody), findsOneWidget);
      expect(
        find.descendant(
          of: key('install-driver'),
          matching: find.text(en.installButton),
        ),
        findsOneWidget,
      );
      await tap(tester, key('install-driver'));
      expect(engine.calls, contains('drivers $cube install'));
      expect(key('driver-notice'), findsNothing);
    });

    testWidgets('Linux with the rule in place shows nothing', (tester) async {
      final engine = FakeEngine(step: Duration.zero);
      await start(tester, engine);
      await toConnect(tester);
      expect(engine.calls, contains('drivers $cube'));
      expect(key('driver-notice'), findsNothing);
    });

    testWidgets('macOS never checks or shows it', (tester) async {
      final engine = FakeEngine(step: Duration.zero, host: 'macos');
      await start(tester, engine);
      await toConnect(tester);
      expect(engine.calls.where((c) => c.startsWith('drivers')), isEmpty);
      expect(key('driver-notice'), findsNothing);
    });
  });

  group('C-6 picked files and the space check', () {
    testWidgets('picked files are passed to space as --import', (tester) async {
      final saved = pickPackFiles;
      addTearDown(() => pickPackFiles = saved);
      pickPackFiles = (_) async => ['/dl/rg.tar.002', '/dl/rg.tar.001'];
      final engine = FakeEngine(step: Duration.zero);
      await start(tester, engine);
      await toBuildStep(tester);
      await tap(tester, key('source-files'));
      expect(
        engine.calls,
        contains('space /ws import /dl/rg.tar.001 /dl/rg.tar.002'),
      );
      // The build-time check (Start build) passes them too.
      engine.calls.clear();
      await tap(tester, key('start-build'));
      expect(
        engine.calls.first,
        'space /ws import /dl/rg.tar.001 /dl/rg.tar.002',
      );
      // Back to downloading: checked again without them.
      engine.calls.clear();
      await tap(tester, key('source-download'));
      expect(engine.calls, ['space /ws']);
    });
  });

  group('C-7 freeing space before a build', () {
    testWidgets('space dialog → clean scratch → Check again starts the build', (
      tester,
    ) async {
      // 15 GB free, 19 GB needed; the scratch folder holds 5.9 GB.
      final engine = FakeEngine(step: Duration.zero, freeBytes: 15000000000);
      await start(tester, engine);
      await toBuildStep(tester);
      await tap(tester, key('start-build'));
      expect(key('space-dialog'), findsOneWidget);
      await tap(tester, key('space-free-up'));
      expect(key('free-up-dialog'), findsOneWidget);
      expect(engine.calls, contains('clean dry'));

      bool checked(String kind) =>
          tester.widget<CheckboxListTile>(key('free-up-kind-$kind')).value!;
      // Temporary files chosen; downloads and other jobs' images offered.
      expect(checked('scratch'), isTrue);
      expect(checked('downloads'), isFalse);
      expect(checked('image'), isFalse);
      expect(find.text('${en.cleanOtherImages} — 6.1 GB'), findsOneWidget);
      // Never backups, and never the image of this device and profile.
      expect(key('free-up-kind-backup'), findsNothing);
      expect(
        find.textContaining('jobs/anbernic-rg-cube-medium/out'),
        findsNothing,
      );
      expect(find.textContaining('/backup/run-'), findsNothing);

      await tap(tester, key('free-up-run'));
      expect(
        engine.calls,
        contains('clean --path /ws/jobs/anbernic-rg-cube-medium/work'),
      );
      expect(key('free-up-freed'), findsOneWidget);
      expect(engine.calls.any((c) => c.startsWith('build')), isFalse);

      await tap(tester, key('free-up-check-again'));
      expect(key('free-up-dialog'), findsNothing);
      expect(key('space-dialog'), findsNothing);
      expect(engine.calls, contains('build $cube medium'));
      expect(key('build-complete'), findsOneWidget);
    });

    testWidgets('the not-enough-space notice offers Free up space too', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero, freeBytes: 15000000000);
      final s = await start(tester, engine);
      await toBuildStep(tester);
      expect(key('space-notice'), findsOneWidget);
      await tap(tester, key('space-notice-free-up'));
      await tap(tester, key('free-up-run'));
      await tap(tester, key('free-up-check-again'));
      // Checked again: enough room now, and no build until Start build.
      expect(s.space?.ok, isTrue);
      expect(key('space-notice'), findsNothing);
      expect(key('space-summary'), findsOneWidget);
      expect(engine.calls.any((c) => c.startsWith('build')), isFalse);
    });

    testWidgets('FAT32: says so, and offers no Free up space', (tester) async {
      final engine = FakeEngine(step: Duration.zero)..spaceReason = 'fat32';
      await start(tester, engine);
      await toBuildStep(tester);
      expect(
        find.descendant(
          of: key('space-notice'),
          matching: find.text(en.spaceFat32),
        ),
        findsOneWidget,
      );
      expect(key('space-notice-free-up'), findsNothing);
      await tap(tester, key('start-build'));
      expect(
        find.descendant(
          of: key('space-dialog'),
          matching: find.text(en.spaceFat32),
        ),
        findsOneWidget,
      );
      expect(key('space-free-up'), findsNothing);
      expect(key('space-choose-folder'), findsOneWidget);
      await tap(tester, key('space-cancel'));
      expect(engine.calls.any((c) => c.startsWith('build')), isFalse);
    });
  });

  group('C-8 deleting backups', () {
    Future<FakeEngine> toDone(WidgetTester tester) async {
      final engine = FakeEngine(step: Duration.zero);
      await start(tester, engine);
      await toReview(tester);
      await install(tester);
      await next(tester);
      await next(tester);
      expect(key('free-up-space'), findsOneWidget);
      return engine;
    }

    testWidgets('lists every backup folder; deletes only the chosen ones, '
        'after confirming', (tester) async {
      final engine = await toDone(tester);
      await tap(tester, key('delete-backups'));
      expect(key('delete-backups-dialog'), findsOneWidget);
      expect(find.text(en.deleteBackupsBody), findsOneWidget);
      // Device, time and path of each.
      expect(
        find.textContaining(
          'anbernic-rg-cube-medium · 2026-11-10 08:15:00 UTC',
        ),
        findsOneWidget,
      );
      expect(
        find.textContaining('Anbernic RG406H · 2026-10-01 19:30:00 UTC'),
        findsOneWidget,
      );
      expect(
        find.text(
          '/ws/jobs/anbernic-rg406h-medium/backup/'
          'run-20261001-193000',
        ),
        findsOneWidget,
      );
      // None chosen, so nothing can be deleted yet.
      bool checked(int i) =>
          tester.widget<CheckboxListTile>(key('backup-$i')).value!;
      expect(checked(0), isFalse);
      expect(checked(1), isFalse);
      expect(
        tester.widget<FilledButton>(key('delete-backups-confirm')).onPressed,
        isNull,
      );
      await tap(tester, key('backup-1'));
      await tap(tester, key('delete-backups-confirm'));
      expect(key('delete-backups-dialog'), findsNothing);
      final cleans = engine.calls.where((c) => c.startsWith('clean --path'));
      expect(cleans, [
        'clean --path /ws/jobs/anbernic-rg406h-medium/backup/'
            'run-20261001-193000',
      ]);
      // The other backup is still there.
      await tap(tester, key('delete-backups'));
      expect(find.textContaining('2026-11-10 08:15:00 UTC'), findsOneWidget);
      expect(find.textContaining('2026-10-01 19:30:00 UTC'), findsNothing);
    });

    testWidgets('Cancel deletes nothing', (tester) async {
      final engine = await toDone(tester);
      await tap(tester, key('delete-backups'));
      await tap(tester, key('backup-0'));
      await tap(tester, key('delete-backups-cancel'));
      expect(engine.calls.where((c) => c.startsWith('clean --path')), isEmpty);
    });

    test(
      'deleteBackups ignores anything that isn\'t a listed backup',
      () async {
        final engine = FakeEngine(step: Duration.zero);
        final s = AppState(engine, onAndroid: false, workspace: '/ws');
        await s.loadCleanItems();
        expect(s.backupItems, hasLength(2));
        expect(
          await s.deleteBackups(['/ws/jobs/anbernic-rg-cube-medium/work']),
          isFalse,
        );
        expect(
          engine.calls.where((c) => c.startsWith('clean --path')),
          isEmpty,
        );
        // The kind-based Done choice can't include backups either.
        s.setCleanChoice('backup', true);
        expect(s.cleanChoice.containsKey('backup'), isFalse);
      },
    );
  });

  group('C-9 the Review partitions row', () {
    testWidgets('switching mode changes the row', (tester) async {
      await start(tester, FakeEngine(step: Duration.zero));
      await toReview(tester, journey: 'returning');
      expect(
        rowText(tester),
        'super, vendor_boot_a, vendor_boot_b, logo\n'
        '${en.partitionsErases('userdata, metadata')}',
      );
      await tap(tester, key('mode-update'));
      expect(rowText(tester), 'super, logo');
      await tap(tester, key('mode-full'));
      expect(rowText(tester), startsWith('super, vendor_boot_a'));
    });

    test('engines without the new fields: the legacy partitions', () {
      final legacy = Plan.fromJson({
        'device': cube,
        'profile': 'medium',
        'partitions': ['super', 'logo'],
      });
      expect(legacy.partitionsFull, isNull);
      expect(partitionsText(en, legacy, FlashMode.full), 'super, logo');
      expect(partitionsText(en, legacy, FlashMode.update), 'super, logo');
    });
  });

  group('C-11 Copy details', () {
    testWidgets('copies the engine log and the run log', (tester) async {
      final engine = _CrashingBuild();
      engine.stderrLog.addAll(['thimsy-installer: panicked at job.rs:42']);
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String?;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      await start(tester, engine);
      await toBuildStep(tester);
      await tap(tester, key('start-build'));
      // The hint points at the button that's there.
      expect(find.text(en.errEngineCrashedHint), findsOneWidget);
      expect(en.errEngineCrashedHint, contains(en.copyDetails));
      await tap(
        tester,
        find.descendant(of: key('run-error'), matching: key('copy-details')),
      );
      expect(copied, contains('panicked at job.rs:42'));
      expect(copied, contains('— Downloading'));
      expect(find.text(en.detailsCopied), findsOneWidget);
    });
  });
}

/// The engine process dies during the build.
class _CrashingBuild extends FakeEngine {
  _CrashingBuild() : super(step: Duration.zero);

  @override
  EngineRun build({
    required String device,
    required String profile,
    required String workspace,
    List<String> imports = const [],
    bool deleteDownloads = false,
  }) => _ListRun([
    const StageEvent('fetch', 'Downloading'),
    const ErrorEvent(
      code: 'engine_crashed',
      message: 'exit code 101',
      local: true,
      args: {'exit': '101'},
    ),
    ResultEvent(false, {'type': 'result', 'ok': false}),
  ]);
}

class _ListRun implements EngineRun {
  final List<EngineEvent> list;
  _ListRun(this.list);
  @override
  Stream<EngineEvent> get events => Stream.fromIterable(list);
  @override
  Future<void> cancel() async {}
}
