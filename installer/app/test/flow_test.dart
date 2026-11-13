import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:thimsy_installer/engine/engine.dart';
import 'package:thimsy_installer/engine/fake_engine.dart';
import 'package:thimsy_installer/l10n/app_localizations.dart';
import 'package:thimsy_installer/main.dart';
import 'package:thimsy_installer/state/app_state.dart';
import 'package:thimsy_installer/widgets/common.dart';

Finder key(String k) => find.byKey(ValueKey(k));

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
  LanguageStore? store,
}) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final state = AppState(engine, onAndroid: false, languageStore: store);
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

/// Device → Profile → Journey → Build, ending on the step after Build.
Future<void> pickAndBuild(
  WidgetTester tester,
  String device, {
  String journey = 'first_time',
}) async {
  await tap(tester, key('device-$device'));
  await next(tester);
  expect(find.text('Choose an experience'), findsOneWidget);
  await next(tester);
  expect(
    find.text('Have you installed custom firmware on this device before?'),
    findsOneWidget,
  );
  expect(tester.widget<FilledButton>(key('next')).onPressed, isNull);
  await tap(tester, key('journey-$journey'));
  await next(tester);
  expect(find.text('Download and build'), findsOneWidget);
  await tester.enterText(key('workspace-field'), '/tmp/thimsy-ws');
  await tester.pumpAndSettle();
  await tap(tester, key('start-build'));
  expect(key('build-complete'), findsOneWidget);
  await next(tester);
}

/// A fake whose flash always says this computer can't install.
class _HostRefusingEngine extends FakeEngine {
  _HostRefusingEngine() : super(step: Duration.zero);

  @override
  EngineRun flash({
    required String job,
    bool dryRun = false,
    FlashMode mode = FlashMode.full,
  }) {
    calls.add('flash ${mode.name}');
    return _Run([
      const ErrorEvent(
        code: 'unsupported_host',
        message: 'This computer can\'t write this device.',
        hint: 'Use a Windows PC',
      ),
      ResultEvent(false, {'type': 'result', 'ok': false}),
    ]);
  }
}

/// A fake whose engine process dies during build, and whose catalog
/// can be made to fail.
class _CrashingEngine extends FakeEngine {
  bool failCatalog;
  _CrashingEngine({this.failCatalog = false}) : super(step: Duration.zero);

  @override
  Future<Catalog> catalog() async {
    if (failCatalog) {
      throw const EngineException(
        ErrorEvent(code: 'no_answer', message: 'x', local: true),
      );
    }
    return super.catalog();
  }

  @override
  EngineRun build({
    required String device,
    required String profile,
    required String workspace,
    List<String> imports = const [],
    bool deleteDownloads = false,
  }) => _Run([
    const StageEvent('fetch', 'Downloading'),
    const ErrorEvent(
      code: 'engine_crashed',
      message: 'exit code 3',
      local: true,
      args: {'exit': '3'},
    ),
    ResultEvent(false, {'type': 'result', 'ok': false}),
  ]);
}

class _Run implements EngineRun {
  final List<EngineEvent> list;
  _Run(this.list);
  @override
  Stream<EngineEvent> get events => Stream.fromIterable(list);
  @override
  Future<void> cancel() async {}
}

void main() {
  testWidgets('full walkthrough with the fake engine', (tester) async {
    final engine = FakeEngine(step: Duration.zero);
    await start(tester, engine);

    expect(find.text('Choose your device'), findsOneWidget);
    expect(find.text('Anbernic RG Cube'), findsOneWidget);
    // Unpublished profiles show as such on the profile step.
    await pickAndBuild(tester, 'anbernic-rg-cube');
    expect(engine.calls, contains('build anbernic-rg-cube medium'));

    // Back up
    expect(find.text('Back up your device'), findsOneWidget);
    expect(tester.widget<FilledButton>(key('next')).onPressed, isNull);
    await tap(tester, key('backed-up'));
    await next(tester);

    // Unlock (guided for the RG Cube)
    expect(find.text('Follow the upstream unlock steps'), findsOneWidget);
    await tap(tester, key('unlock-confirm'));
    await next(tester);

    // Connect: the fake reports the device in download mode.
    expect(key('device-connected'), findsOneWidget);
    await next(tester);

    // Review → Install: a first-time install offers only a fresh install.
    expect(find.text('Fresh install (erases everything)'), findsOneWidget);
    expect(key('mode-update'), findsNothing);
    expect(find.text('All data on the device will be erased'), findsOneWidget);
    await tap(tester, key('install-button'));
    expect(key('confirm-install'), findsOneWidget);
    await tap(tester, key('confirm-install-yes'));
    expect(key('install-complete'), findsOneWidget);
    expect(engine.calls, contains('flash full'));
    // Leaving Connect stopped the detect watch; it ran once.
    expect(engine.calls.where((c) => c == 'detect'), hasLength(1));

    await next(tester);
    expect(find.text('First boot'), findsWidgets);
    await next(tester);
    expect(find.text('Thimsy is installed'), findsOneWidget);
  });

  testWidgets('profile step shows unpublished profiles disabled', (
    tester,
  ) async {
    await start(tester, FakeEngine(step: Duration.zero));
    await tap(tester, key('device-anbernic-rg-cube'));
    await next(tester);
    expect(find.text('Not published yet'), findsNWidgets(2));
    expect(find.textContaining('ARMSX2: Heavy profile only'), findsOneWidget);
  });

  testWidgets('needs-Windows device: dialog, then build and export', (
    tester,
  ) async {
    final engine = FakeEngine(step: Duration.zero, host: 'macos');
    await start(tester, engine);

    expect(key('needs-windows-anbernic-rg557'), findsOneWidget);
    expect(find.text('Needs Windows'), findsWidgets);

    await tap(tester, key('device-anbernic-rg557'));
    await next(tester);
    expect(key('needs-windows-dialog'), findsOneWidget);
    expect(
      find.textContaining('SP Flash Tool, which runs on Windows and Linux'),
      findsOneWidget,
    );
    await tap(tester, find.text('Build and export anyway'));
    expect(find.text('Choose an experience'), findsOneWidget);

    await next(tester);
    expect(find.text('Build and export only'), findsOneWidget);
    await tester.enterText(key('workspace-field'), '/tmp/thimsy-ws');
    await tester.pumpAndSettle();
    await tap(tester, key('start-build'));
    await next(tester);

    // Flash steps are skipped: straight to Done with the Windows note.
    expect(key('finish-on-windows'), findsOneWidget);
    expect(find.text('Image ready'), findsOneWidget);
    expect(key('rail-install'), findsNothing);
    expect(engine.calls.any((c) => c.startsWith('flash')), isFalse);
  });

  testWidgets('needs-Windows device without a verified build: no export', (
    tester,
  ) async {
    await start(tester, FakeEngine(step: Duration.zero, host: 'macos'));
    await tap(tester, key('device-gamemt-e6-max'));
    await next(tester);
    expect(key('needs-windows-dialog'), findsOneWidget);
    expect(
      find.textContaining('can only be installed from a Windows PC'),
      findsOneWidget,
    );
    expect(find.text('Build and export anyway'), findsNothing);
    await tap(tester, find.text('OK'));
    expect(find.text('Choose your device'), findsOneWidget);
  });

  testWidgets('works-here filter hides devices this computer cannot do', (
    tester,
  ) async {
    await start(tester, FakeEngine(step: Duration.zero, host: 'macos'));
    expect(key('device-anbernic-rg557'), findsOneWidget);
    await tap(tester, key('works-here'));
    expect(key('device-anbernic-rg557'), findsNothing);
    expect(key('device-retroid-pocket-4-pro'), findsOneWidget);
  });

  testWidgets('engine unsupported_host on flash shows the same dialog', (
    tester,
  ) async {
    final engine = _HostRefusingEngine();
    await start(tester, engine);
    await pickAndBuild(tester, 'anbernic-rg-cube', journey: 'returning');
    await tap(tester, key('backed-up'));
    await next(tester);
    await next(tester); // connected (unlock skipped)
    await tap(tester, key('install-button'));
    await tap(tester, key('confirm-install-yes'));

    expect(engine.calls, contains('flash full'));
    expect(key('needs-windows-dialog'), findsOneWidget);
    // The catalog's translated text, not the engine's English message.
    expect(
      find.textContaining('can only be installed from a Windows PC'),
      findsOneWidget,
    );
    expect(find.text('This computer can\'t write this device.'), findsNothing);
    await tap(tester, find.text('Build and export anyway'));
    expect(key('finish-on-windows'), findsOneWidget);
  });

  testWidgets('returning journey skips unlock and installs full by default', (
    tester,
  ) async {
    final engine = FakeEngine(step: Duration.zero);
    await start(tester, engine);
    await pickAndBuild(tester, 'anbernic-rg-cube', journey: 'returning');
    expect(key('rail-unlock'), findsNothing);
    await tap(tester, key('backed-up'));
    await next(tester);
    // Straight to Connect: no unlock screen.
    expect(find.text('Unlock the bootloader'), findsNothing);
    expect(find.text('Connect your device'), findsOneWidget);
    expect(key('device-connected'), findsOneWidget);
    await next(tester);

    // Update is offered with what it needs, but full is chosen: the app
    // can't see whether the device runs a matching base.
    expect(find.text('Update (keep my data)'), findsOneWidget);
    expect(find.text('Reinstall (erase everything)'), findsOneWidget);
    expect(
      find.textContaining('GammaOS Next 1.2-based firmware'),
      findsOneWidget,
    );
    expect(find.text('All data on the device will be erased'), findsOneWidget);
    expect(key('update-requirement'), findsNothing);
    // Full is listed first.
    expect(
      tester.getTopLeft(key('mode-full')).dy,
      lessThan(tester.getTopLeft(key('mode-update')).dy),
    );
    await tap(tester, key('install-button'));
    await tap(tester, key('confirm-install-yes'));
    expect(engine.calls, contains('flash full'));
    expect(engine.calls, isNot(contains('flash update')));
    expect(key('install-complete'), findsOneWidget);
  });

  testWidgets('returning journey can still choose update', (tester) async {
    final engine = FakeEngine(step: Duration.zero);
    await start(tester, engine);
    await pickAndBuild(tester, 'anbernic-rg-cube', journey: 'returning');
    await tap(tester, key('backed-up'));
    await next(tester);
    await next(tester);
    await tap(tester, key('mode-update'));
    expect(find.text('All data on the device will be erased'), findsNothing);
    // What an update needs, and what happens otherwise, is spelled out.
    expect(key('update-requirement'), findsOneWidget);
    expect(
      find.textContaining('nothing is written and you can switch'),
      findsOneWidget,
    );
    await tap(tester, key('install-button'));
    await tap(tester, key('confirm-install-yes'));
    expect(engine.calls, contains('flash update'));
  });

  testWidgets('first-time journey shows the unlock step', (tester) async {
    final engine = FakeEngine(step: Duration.zero);
    await start(tester, engine);
    await pickAndBuild(tester, 'anbernic-rg-cube');
    expect(key('rail-unlock'), findsOneWidget);
    await tap(tester, key('backed-up'));
    await next(tester);
    expect(find.text('Unlock the bootloader'), findsOneWidget);
  });

  testWidgets('dry run without a device', (tester) async {
    final engine = FakeEngine(step: Duration.zero, detectDevice: 'other');
    await start(tester, engine);
    await pickAndBuild(tester, 'anbernic-rg-cube');
    await tap(tester, key('backed-up'));
    await next(tester);
    await tap(tester, key('unlock-confirm'));
    await next(tester);
    expect(key('device-connected'), findsNothing);
    await tap(tester, key('dry-run-continue'));
    expect(find.text('Review'), findsWidgets);
    await tap(tester, key('install-button'));
    await tap(tester, key('confirm-install-yes'));
    expect(engine.calls, contains('flash full dry'));
    expect(find.text('Dry run complete'), findsOneWidget);

    // A dry run can't turn into a real install from the install screen:
    // no switch there, and "Run again" repeats the dry run.
    expect(find.byType(SwitchListTile), findsNothing);
    await tap(tester, key('run-again'));
    expect(engine.calls.where((c) => c == 'flash full dry').length, 2);
    expect(engine.calls.where((c) => c == 'flash full'), isEmpty);

    // Back on Review, turning dry run off doesn't allow a real install of a
    // device that must be connected first and wasn't detected.
    await tap(tester, key('back-to-review'));
    await tap(tester, find.text('Advanced'));
    await tap(tester, key('dry-run'));
    final install = tester.widget<ButtonStyleButton>(key('install-button'));
    expect(install.onPressed, isNull);
    expect(engine.calls.where((c) => c == 'flash full'), isEmpty);
  });

  testWidgets('Unisoc: connect after installing starts', (tester) async {
    final engine = FakeEngine(
      step: Duration.zero,
      detectDevice: 'other',
      connectFirst: false,
    );
    await start(tester, engine);
    await pickAndBuild(tester, 'anbernic-rg-cube');
    await tap(tester, key('backed-up'));
    await next(tester);
    await tap(tester, key('unlock-confirm'));
    await next(tester);
    // Nothing detected, and that's expected: the tool will wait for it.
    // The page sets up USB and shows the steps as what comes after Install,
    // instead of asking to plug in now.
    expect(key('connect-later'), findsOneWidget);
    expect(key('device-connected'), findsNothing);
    expect(find.text('Get ready to connect'), findsOneWidget);
    expect(find.text('Connect your device'), findsNothing);
    expect(key('connect-steps-later'), findsOneWidget);
    expect(key('dry-run-continue'), findsNothing);
    await next(tester);
    expect(find.text('Review'), findsWidgets);
    expect(
      find.descendant(
        of: key('row-connected-as'),
        matching: find.text('After you press Install'),
      ),
      findsOneWidget,
    );
    expect(key('install-then-connect'), findsOneWidget);
    expect(key('image-ready'), findsOneWidget);
    expect(engine.calls, contains('prepare full'));
    await tap(tester, key('install-button'));
    await tap(tester, key('confirm-install-yes'));
    expect(engine.calls, contains('flash full'));
  });

  test('automated unlock runs the engine', () async {
    final engine = FakeEngine(step: Duration.zero, detectDevice: 'x');
    final state = AppState(engine, onAndroid: false);
    await state.loadCatalog();
    state.selectDevice('retroid-pocket-4-pro');
    expect(state.deviceNext(), DeviceNext.proceeded);
    await state.loadPlans();
    state.selectJourney('first_time');
    expect(state.unlockRoute, 'automated');
    expect(state.isSkipped(WizardStep.unlock), isFalse);
    state.startUnlock();
    for (var i = 0; i < 100 && !state.unlockRun.ok; i++) {
      await Future<void>.delayed(const Duration(milliseconds: 5));
    }
    expect(state.unlockRun.ok, isTrue);
    expect(engine.calls, contains('unlock retroid-pocket-4-pro'));
  });

  group('disk space', () {
    testWidgets('not enough space: dialog, and the build is not started', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero, freeBytes: 5000000000);
      await start(tester, engine);
      await tap(tester, key('device-anbernic-rg-cube'));
      await next(tester);
      await next(tester);
      await tap(tester, key('journey-first_time'));
      await next(tester);
      await tester.enterText(key('workspace-field'), 'D:\\thimsy');
      await tester.pumpAndSettle();
      // The check runs when the workspace changes and shows need / free.
      expect(key('space-notice'), findsOneWidget);
      expect(
        find.descendant(
          of: key('space-notice'),
          matching: find.textContaining(
            'needs 19.0 GB, 5.0 GB available on D:',
          ),
        ),
        findsOneWidget,
      );

      await tap(tester, key('start-build'));
      expect(key('space-dialog'), findsOneWidget);
      expect(
        find.descendant(
          of: key('space-dialog'),
          matching: find.text(
            'Not enough free space: needs 19.0 GB, 5.0 GB available on D:. '
            'Choose another folder or free up space.',
          ),
        ),
        findsOneWidget,
      );
      expect(key('space-choose-folder'), findsOneWidget);
      expect(engine.calls.any((c) => c.startsWith('build')), isFalse);

      await tap(tester, key('space-cancel'));
      expect(key('space-dialog'), findsNothing);
      expect(engine.calls.any((c) => c.startsWith('build')), isFalse);

      // Check again after freeing space: the build goes ahead.
      await tap(tester, key('start-build'));
      expect(key('space-dialog'), findsOneWidget);
      engine.freeBytes = 50000000000;
      await tap(tester, key('space-check-again'));
      expect(key('space-dialog'), findsNothing);
      expect(engine.calls, contains('build anbernic-rg-cube medium'));
      expect(key('build-complete'), findsOneWidget);
    });

    testWidgets('choose another folder re-checks the new folder', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero, freeBytes: 1000000000);
      pickDirectory = () async => '/mnt/big';
      addTearDown(() => pickDirectory = () async => null);
      await start(tester, engine);
      await tap(tester, key('device-anbernic-rg-cube'));
      await next(tester);
      await next(tester);
      await tap(tester, key('journey-first_time'));
      await next(tester);
      await tester.enterText(key('workspace-field'), '/small');
      await tester.pumpAndSettle();
      await tap(tester, key('start-build'));
      engine.freeBytes = 90000000000;
      await tap(tester, key('space-choose-folder'));
      expect(engine.calls, contains('space /mnt/big'));
      expect(find.text('/mnt/big'), findsOneWidget);
      expect(engine.calls.any((c) => c.startsWith('build')), isFalse);
    });

    testWidgets('delete downloads is passed to space and build', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero);
      await start(tester, engine);
      await tap(tester, key('device-anbernic-rg-cube'));
      await next(tester);
      await next(tester);
      await tap(tester, key('journey-first_time'));
      await next(tester);
      await tester.enterText(key('workspace-field'), '/ws');
      await tester.pumpAndSettle();
      expect(find.textContaining('saves about 2.7 GB'), findsOneWidget);
      await tap(tester, key('delete-downloads'));
      expect(engine.calls, contains('space /ws delete'));
      await tap(tester, key('start-build'));
      expect(engine.calls, contains('build anbernic-rg-cube medium delete'));
    });

    testWidgets('flash not_enough_space: dialog, check again retries', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero, flashSpaceFailures: 1);
      await start(tester, engine);
      await pickAndBuild(tester, 'anbernic-rg-cube');
      await tap(tester, key('backed-up'));
      await next(tester);
      await tap(tester, key('unlock-confirm'));
      await next(tester);
      await next(tester);
      await tap(tester, key('install-button'));
      await tap(tester, key('confirm-install-yes'));
      expect(key('space-dialog'), findsOneWidget);
      // The engine's (translated) hint; its English message under Details.
      expect(
        find.textContaining('Free up space on the workspace drive'),
        findsOneWidget,
      );
      expect(find.textContaining('prepare the partitions'), findsNothing);
      await tap(tester, key('error-details'));
      expect(find.textContaining('prepare the partitions'), findsOneWidget);
      expect(key('space-choose-folder'), findsNothing);
      await tap(tester, key('space-check-again'));
      expect(engine.calls.where((c) => c == 'flash full'), hasLength(2));
      expect(key('install-complete'), findsOneWidget);
    });

    testWidgets('done screen frees up space with sensible defaults', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero);
      await start(tester, engine);
      await pickAndBuild(tester, 'anbernic-rg-cube', journey: 'returning');
      await tap(tester, key('backed-up'));
      await next(tester);
      await next(tester);
      await tap(tester, key('install-button'));
      await tap(tester, key('confirm-install-yes'));
      await next(tester);
      await next(tester);
      expect(find.text('Thimsy is installed'), findsOneWidget);
      expect(engine.calls, contains('clean dry'));
      bool checked(String kind) =>
          tester.widget<CheckboxListTile>(key('clean-$kind')).value!;
      expect(checked('scratch'), isTrue);
      expect(checked('downloads'), isTrue);
      expect(checked('image'), isFalse);
      // Backups are never a checkbox here: only "Delete backups…".
      expect(key('clean-backup'), findsNothing);
      expect(key('delete-backups'), findsOneWidget);
      expect(find.textContaining('calibration data'), findsWidgets);
      expect(find.text('Free up 8.6 GB'), findsOneWidget);
      await tap(tester, key('run-clean'));
      // Only the listed items, by path.
      expect(
        engine.calls,
        contains(
          'clean --path /tmp/thimsy-ws/cache/RG_Cube_GammaOS_Next_Full_v1.2.7z.001'
          ' --path /tmp/thimsy-ws/jobs/anbernic-rg-cube-medium/work',
        ),
      );
      expect(engine.calls.where((c) => c.contains('backup')), isEmpty);
      expect(find.text('Freed 8.6 GB.'), findsOneWidget);
    });
  });

  group('languages', () {
    testWidgets('picker switches language, persists it and tells the engine', (
      tester,
    ) async {
      final engine = FakeEngine(step: Duration.zero);
      final store = MemoryStore();
      await start(tester, engine, store: store);
      expect(engine.language, 'en');
      await tap(tester, key('language-button'));
      expect(key('language-dialog'), findsOneWidget);
      expect(find.text('Deutsch'), findsOneWidget);
      expect(find.text('繁體中文'), findsOneWidget);
      await tap(tester, key('lang-de'));
      expect(find.text('Wählen Sie Ihr Gerät'), findsOneWidget);
      expect(store.value, 'de');
      expect(engine.language, 'de');
      // Engine text is fetched again in the new language.
      expect(engine.calls.where((c) => c == 'catalog'), hasLength(2));

      await tap(tester, key('language-button'));
      await tap(tester, key('lang-system'));
      expect(find.text('Choose your device'), findsOneWidget);
      expect(store.value, isNull);
    });

    testWidgets('a saved language is used at start (zh-tw)', (tester) async {
      final engine = FakeEngine(step: Duration.zero);
      await start(tester, engine, store: MemoryStore('zh-tw'));
      expect(find.text('選擇您的裝置'), findsOneWidget);
      expect(engine.language, 'zh-tw');
    });

    testWidgets('Arabic renders right-to-left', (tester) async {
      final engine = FakeEngine(step: Duration.zero);
      await start(tester, engine, store: MemoryStore('ar'));
      final title = find.text('اختر جهازك');
      expect(title, findsOneWidget);
      expect(Directionality.of(tester.element(title)), TextDirection.rtl);
      // The step rail sits on the right in RTL.
      final rail = tester.getTopLeft(key('rail-device'));
      final content = tester.getTopLeft(title);
      expect(rail.dx, greaterThan(content.dx));
      expect(engine.language, 'ar');
      await tap(tester, key('device-anbernic-rg-cube'));
      await next(tester);
      expect(find.text('اختر تجربة'), findsOneWidget);
    });

    testWidgets('Hebrew is right-to-left too', (tester) async {
      await start(
        tester,
        FakeEngine(step: Duration.zero),
        store: MemoryStore('he'),
      );
      final title = find.text('בחרו את המכשיר שלכם');
      expect(Directionality.of(tester.element(title)), TextDirection.rtl);
    });

    testWidgets('Japanese renders the device step', (tester) async {
      final engine = FakeEngine(step: Duration.zero);
      await start(tester, engine, store: MemoryStore('ja'));
      final title = find.text('デバイスを選択');
      expect(title, findsOneWidget);
      expect(Directionality.of(tester.element(title)), TextDirection.ltr);
      expect(find.text('このコンピューターでインストールできるデバイスのみ'), findsOneWidget);
    });
  });

  testWidgets('app-raised engine errors are shown translated', (tester) async {
    final engine = _CrashingEngine(failCatalog: true);
    await start(tester, engine, store: MemoryStore('fr'));
    expect(
      find.text('Impossible de charger la liste des appareils'),
      findsOneWidget,
    );
    expect(
      find.text("Le moteur d'installation n'a pas répondu."),
      findsOneWidget,
    );
    engine.failCatalog = false;
    await tap(tester, find.text('Réessayer'));
    await tap(tester, key('device-anbernic-rg-cube'));
    await next(tester);
    await next(tester);
    await tap(tester, key('journey-first_time'));
    await next(tester);
    await tester.enterText(key('workspace-field'), '/ws');
    await tester.pumpAndSettle();
    await tap(tester, key('start-build'));
    expect(
      find.text(
        "Le moteur d'installation s'est arrêté de façon inattendue (code de sortie 3).",
      ),
      findsOneWidget,
    );
    // The hint names the Copy details button next to it.
    final fr = lookupAppLocalizations(const Locale('fr'));
    expect(find.text(fr.errEngineCrashedHint), findsOneWidget);
    expect(fr.errEngineCrashedHint, contains(fr.copyDetails));
    expect(
      find.descendant(
        of: key('run-error'),
        matching: find.text(fr.copyDetails),
      ),
      findsOneWidget,
    );
  });

  testWidgets('Android shows the later-version notice', (tester) async {
    await tester.pumpWidget(
      ThimsyInstallerApp(
        state: AppState(FakeEngine(step: Duration.zero), onAndroid: true),
      ),
    );
    await tester.pumpAndSettle();
    expect(key('android-notice'), findsOneWidget);
    expect(
      find.text('Building and flashing from Android comes in a later version'),
      findsOneWidget,
    );
  });
}
