// Guards against using a stale build, repeating an unconfirmed install,
// stopping a real install by accident, and mixing plans or detected devices
// from an earlier selection.
import 'dart:async';
import 'dart:ui' show AppExitResponse;

import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:thimsy_installer/engine/engine.dart';
import 'package:thimsy_installer/engine/fake_engine.dart';
import 'package:thimsy_installer/main.dart';
import 'package:thimsy_installer/screens/home_shell.dart';
import 'package:thimsy_installer/state/app_state.dart';

Finder key(String k) => find.byKey(ValueKey(k));

const cube = 'anbernic-rg-cube';
const rp4 = 'retroid-pocket-4-pro';

/// A run whose events the test pushes. [cancel] only records the request:
/// like a real process, it can still emit a few lines before it's gone.
class HeldRun implements EngineRun {
  final StreamController<EngineEvent> ctl = StreamController();
  bool cancelled = false;

  void add(EngineEvent e) => ctl.add(e);
  Future<void> finish([EngineEvent? last]) async {
    if (last != null) ctl.add(last);
    await ctl.close();
  }

  @override
  Stream<EngineEvent> get events => ctl.stream;

  @override
  Future<void> cancel() async => cancelled = true;
}

/// The fake engine, with builds, flashes, space checks and plan resolves the
/// test can hold and release.
class ControlledEngine extends FakeEngine {
  ControlledEngine({super.host}) : super(step: Duration.zero);

  bool holdBuild = false;
  bool holdFlash = false;
  bool holdDetect = false;
  bool holdUnlock = false;
  final List<HeldRun> builds = [];
  final List<HeldRun> flashes = [];
  final List<HeldRun> detects = [];
  final List<HeldRun> unlocks = [];

  /// When set, `drivers` waits for it.
  Completer<void>? driverGate;

  /// The engine language each `resolve` was made in.
  final List<String> resolveLanguages = [];

  /// When set, `space` waits for it.
  Completer<void>? spaceGate;

  /// Per device: `resolve` waits for it.
  final Map<String, Completer<void>> resolveGates = {};

  /// Flashes that fail (after their check stage) instead of working.
  int failFlashes = 0;

  @override
  Future<Plan> resolve({
    required String device,
    required String profile,
    Map<String, String> overrides = const {},
  }) async {
    resolveLanguages.add(language);
    await resolveGates[device]?.future;
    return super.resolve(device: device, profile: profile);
  }

  @override
  EngineRun detect() {
    if (!holdDetect) return super.detect();
    calls.add('detect');
    final run = HeldRun();
    detects.add(run);
    return run;
  }

  @override
  EngineRun unlock({required String device}) {
    if (!holdUnlock) return super.unlock(device: device);
    calls.add('unlock $device');
    final run = HeldRun();
    unlocks.add(run);
    return run;
  }

  @override
  Future<ResultEvent> drivers({
    required String device,
    bool install = false,
  }) async {
    await driverGate?.future;
    return super.drivers(device: device, install: install);
  }

  @override
  Future<SpaceInfo> space({
    required String device,
    required String profile,
    required String workspace,
    bool deleteDownloads = false,
  }) async {
    await spaceGate?.future;
    return super.space(
      device: device,
      profile: profile,
      workspace: workspace,
      deleteDownloads: deleteDownloads,
    );
  }

  @override
  EngineRun build({
    required String device,
    required String profile,
    required String workspace,
    List<String> imports = const [],
    bool deleteDownloads = false,
  }) {
    if (!holdBuild) {
      return super.build(
        device: device,
        profile: profile,
        workspace: workspace,
        imports: imports,
        deleteDownloads: deleteDownloads,
      );
    }
    calls.add('build $device $profile');
    final run = HeldRun();
    builds.add(run);
    return run;
  }

  @override
  EngineRun flash({
    required String job,
    bool dryRun = false,
    FlashMode mode = FlashMode.full,
  }) {
    if (failFlashes > 0) {
      failFlashes--;
      calls.add('flash ${mode.name}${dryRun ? ' dry' : ''}');
      final run = HeldRun();
      run.add(const StageEvent('check', 'Checking the image'));
      run.add(const StageEvent('flash', 'Writing Thimsy'));
      run.add(const ErrorEvent(code: 'flash_failed', message: 'USB error'));
      run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
      return run;
    }
    if (!holdFlash) return super.flash(job: job, dryRun: dryRun, mode: mode);
    calls.add('flash ${mode.name}${dryRun ? ' dry' : ''}');
    final run = HeldRun();
    flashes.add(run);
    return run;
  }
}

ResultEvent buildOk(String device) => ResultEvent(true, {
  'type': 'result',
  'ok': true,
  'job': '/ws/jobs/$device-medium',
  'image': '/ws/jobs/$device-medium/out/Thimsy.pac',
});

/// Lets the fake engine's zero-length timers and microtasks run.
Future<void> settle() => Future<void>.delayed(const Duration(milliseconds: 20));

/// Waits (up to 2 s) for [condition], for steps that take several turns.
Future<void> until(bool Function() condition) async {
  for (var i = 0; i < 400 && !condition(); i++) {
    await Future<void>.delayed(const Duration(milliseconds: 5));
  }
}

/// Device → Profile → Journey → Build, as state calls.
Future<AppState> toBuild(
  ControlledEngine engine, {
  String device = cube,
}) async {
  final s = AppState(engine, onAndroid: false, workspace: '/ws');
  await s.loadCatalog();
  await s.selectDevice(device);
  expect(s.deviceNext(), DeviceNext.proceeded);
  await settle();
  s.next(); // profile → journey
  s.selectJourney('first_time');
  s.next();
  expect(s.step, WizardStep.build);
  await settle();
  return s;
}

/// Build (done) → Backup → Unlock → Connect → Review, as state calls.
Future<void> toReviewState(AppState s) async {
  expect(s.buildCurrent, isTrue);
  s.next(); // backup
  s.setBackedUp(true);
  s.next(); // unlock
  s.setUnlockConfirmed(true);
  s.next(); // connect: the watch starts
  await settle();
  s.next(); // review
  expect(s.step, WizardStep.review);
}

// ------------------------------------------------------------ widget helpers

Future<AppState> startApp(
  WidgetTester tester,
  FakeEngine engine, {
  bool? guardWindowClose,
}) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final state = AppState(engine, onAndroid: false);
  await tester.pumpWidget(
    ThimsyInstallerApp(state: state, guardWindowClose: guardWindowClose),
  );
  await tester.pumpAndSettle();
  return state;
}

Future<void> tap(WidgetTester tester, Finder f) async {
  await tester.ensureVisible(f);
  await tester.pumpAndSettle();
  await tester.tap(f);
  await tester.pumpAndSettle();
}

/// For screens with a running job: its progress bar never settles.
Future<void> pumpFor(WidgetTester tester) async {
  for (var i = 0; i < 5; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
}

Future<void> tapRunning(WidgetTester tester, Finder f) async {
  await tester.ensureVisible(f);
  await pumpFor(tester);
  await tester.tap(f);
  await pumpFor(tester);
}

Future<void> next(WidgetTester tester) => tap(tester, key('next'));

/// Device → … → Review (real install, device connected), for [journey].
Future<void> toReview(
  WidgetTester tester, {
  String journey = 'returning',
}) async {
  await tap(tester, key('device-$cube'));
  await next(tester);
  await next(tester);
  await tap(tester, key('journey-$journey'));
  await next(tester);
  await tester.enterText(key('workspace-field'), '/tmp/thimsy-ws');
  await tester.pumpAndSettle();
  await tap(tester, key('start-build'));
  expect(key('build-complete'), findsOneWidget);
  await next(tester);
  await tap(tester, key('backed-up'));
  await next(tester);
  if (journey == 'first_time') {
    await tap(tester, key('unlock-confirm'));
    await next(tester);
  }
  expect(key('device-connected'), findsOneWidget);
  await next(tester);
  expect(key('install-button'), findsOneWidget);
}

void main() {
  group('a build for another selection is never used', () {
    test('while a build runs, the selection and steps are locked', () async {
      final engine = ControlledEngine()..holdBuild = true;
      final s = await toBuild(engine);
      await s.startBuild();
      expect(s.buildRun.running, isTrue);
      expect(s.busy, isTrue);

      await s.selectDevice(rp4);
      expect(s.deviceId, cube);
      s.selectJourney('returning');
      expect(s.journeyId, 'first_time');
      s.goTo(WizardStep.device);
      expect(s.step, WizardStep.build);
      expect(s.canGoTo(WizardStep.profile), isFalse);

      await engine.builds.single.finish(buildOk(cube));
      await settle();
      expect(s.busy, isFalse);
      expect(s.canProceed(WizardStep.build), isTrue);
      expect(s.canGoTo(WizardStep.profile), isTrue);
    });

    test('events from a replaced build are dropped', () async {
      final engine = ControlledEngine()..holdBuild = true;
      final s = await toBuild(engine);
      await s.startBuild();
      final runA = engine.builds.single;
      runA.add(const StageEvent('fetch', 'Downloading'));
      // A's result arrives, but its stream hasn't closed yet.
      runA.add(buildOk(cube));
      await settle();
      expect(s.job, isNotNull);

      // B is chosen: A's build is stopped and its leftovers ignored.
      s.goTo(WizardStep.device);
      await s.selectDevice(rp4);
      expect(runA.cancelled, isTrue);
      runA.add(const LogEvent('info', 'late line'));
      await runA.finish(buildOk(cube));
      await settle();

      expect(s.buildRun.ok, isFalse);
      expect(s.buildRun.log, isEmpty);
      expect(s.job, isNull);
      expect(s.canProceed(WizardStep.build), isFalse);
      expect(s.furthest.index, lessThan(WizardStep.build.index));
    });

    test('a device change during the space check starts no build', () async {
      final engine = ControlledEngine();
      final s = await toBuild(engine);
      engine.spaceGate = Completer<void>();
      final starting = s.startBuild();
      await settle();
      s.goTo(WizardStep.device);
      await s.selectDevice(rp4);
      engine.spaceGate!.complete();
      await starting;
      await settle();
      expect(engine.calls.where((c) => c.startsWith('build')), isEmpty);
      expect(s.buildRun.running, isFalse);
      expect(s.busy, isFalse);
      expect(s.job, isNull);
      // The old check's answer isn't shown for the new device.
      expect(s.space, isNull);
    });

    testWidgets('the rail and Back are disabled while building', (
      tester,
    ) async {
      final engine = ControlledEngine()..holdBuild = true;
      await startApp(tester, engine);
      await tap(tester, key('device-$cube'));
      await next(tester);
      await next(tester);
      await tap(tester, key('journey-first_time'));
      await next(tester);
      await tester.enterText(key('workspace-field'), '/tmp/thimsy-ws');
      await tester.pumpAndSettle();
      await tapRunning(tester, key('start-build'));
      expect(engine.builds, hasLength(1));

      bool railEnabled(String step) =>
          tester.widget<ListTile>(key('rail-$step')).enabled;
      expect(railEnabled('device'), isFalse);
      expect(railEnabled('profile'), isFalse);
      expect(railEnabled('journey'), isFalse);
      expect(tester.widget<TextButton>(key('back')).onPressed, isNull);
      expect(tester.widget<FilledButton>(key('next')).onPressed, isNull);
      await tester.tap(key('rail-device'), warnIfMissed: false);
      await pumpFor(tester);
      expect(find.text('Download and build'), findsOneWidget);

      await engine.builds.single.finish(buildOk(cube));
      await tester.pumpAndSettle();
      expect(railEnabled('device'), isTrue);
      expect(tester.widget<FilledButton>(key('next')).onPressed, isNotNull);
    });
  });

  group('Run again repeats only what was confirmed', () {
    testWidgets('after a failed update, a journey change allows no rerun', (
      tester,
    ) async {
      final engine = ControlledEngine()..failFlashes = 1;
      final s = await startApp(tester, engine);
      await toReview(tester);
      await tap(tester, key('install-button'));
      await tap(tester, key('confirm-install-yes'));
      expect(engine.calls.where((c) => c.startsWith('flash')), [
        'flash update',
      ]);
      expect(key('run-again'), findsOneWidget);

      // Change the journey, then try to open Install from the rail.
      await tap(tester, key('rail-journey'));
      await tap(tester, key('journey-first_time'));
      expect(s.flashRun.result, isNull);
      expect(s.canRunAgain, isFalse);
      expect(tester.widget<ListTile>(key('rail-install')).enabled, isFalse);
      await tester.tap(key('rail-install'), warnIfMissed: false);
      await tester.pumpAndSettle();
      expect(key('run-again'), findsNothing);
      // Even called directly, it only goes back to Review.
      s.runAgain();
      await tester.pumpAndSettle();
      expect(engine.calls.where((c) => c.startsWith('flash')), [
        'flash update',
      ]);
      expect(engine.calls, isNot(contains('flash full')));
    });

    testWidgets('a mode change on Review clears the failed install', (
      tester,
    ) async {
      final engine = ControlledEngine()..failFlashes = 1;
      final s = await startApp(tester, engine);
      await toReview(tester);
      await tap(tester, key('install-button'));
      await tap(tester, key('confirm-install-yes'));
      expect(s.canRunAgain, isTrue);
      await tap(tester, key('back-to-review'));
      await tap(tester, key('mode-full'));
      expect(s.canRunAgain, isFalse);
      expect(s.flashRun.result, isNull);
      expect(s.furthest, WizardStep.review);
      // The new mode needs its own confirmation.
      await tap(tester, key('install-button'));
      expect(key('confirm-install'), findsOneWidget);
      await tap(tester, key('confirm-install-yes'));
      expect(engine.calls.where((c) => c.startsWith('flash')), [
        'flash update',
        'flash full',
      ]);
    });

    /// A dry run that stopped on not enough space (the dialog is up).
    Future<(FakeEngine, AppState)> drySpaceFailure() async {
      final engine = FakeEngine(step: Duration.zero, flashSpaceFailures: 1);
      final s = AppState(engine, onAndroid: false, workspace: '/ws');
      await s.loadCatalog();
      await s.selectDevice(cube);
      s.deviceNext();
      await settle();
      s.next();
      s.selectJourney('first_time');
      s.next();
      await s.startBuild();
      await settle();
      expect(s.job, isNotNull);
      s.setDryRun(true);
      s.install();
      await settle();
      expect(s.spaceProblem?.kind, SpaceProblemKind.flash);
      return (engine, s);
    }

    test('Check again after a flash space error repeats the dry run', () async {
      final (engine, s) = await drySpaceFailure();
      await s.recheckSpace();
      await settle();
      expect(engine.calls.where((c) => c.startsWith('flash')), [
        'flash full dry',
        'flash full dry',
      ]);
      expect(s.flashRun.ok, isTrue);
      expect(s.flashRunDry, isTrue);
    });

    test(
      'Check again after dry run was switched off goes back to Review',
      () async {
        final (engine, s) = await drySpaceFailure();
        // Switched off afterwards: neither the dry run nor a real install
        // (never confirmed) is started; Review asks again.
        s.setDryRun(false);
        expect(s.canRunAgain, isFalse);
        await s.recheckSpace();
        await settle();
        expect(engine.calls.where((c) => c.startsWith('flash')), [
          'flash full dry',
        ]);
        expect(s.step, WizardStep.review);
        expect(s.flashRun.result, isNull);
      },
    );

    test('turning Dry run on clears a confirmed real install', () async {
      final engine = ControlledEngine();
      final s = await toBuild(engine);
      await s.startBuild();
      await settle();
      await toReviewState(s);
      s.install();
      await settle();
      expect(engine.calls, contains('flash full'));
      expect(s.flashRun.ok, isTrue);
      expect(s.canProceed(WizardStep.install), isTrue);
      expect(s.canRunAgain, isTrue);

      s.goTo(WizardStep.review);
      s.setDryRun(true);
      expect(s.flashRun.result, isNull);
      expect(s.canRunAgain, isFalse);
      expect(s.canProceed(WizardStep.install), isFalse);
      expect(s.furthest, WizardStep.review);
      // Setting it to the confirmed value again doesn't bring it back.
      s.setDryRun(false);
      expect(s.canRunAgain, isFalse);
      expect(s.flashRun.result, isNull);
    });
  });

  group('Stop during a real install', () {
    Future<HeldRun> startInstall(
      WidgetTester tester,
      ControlledEngine engine, {
      bool dry = false,
      bool? guardWindowClose,
    }) async {
      await startApp(tester, engine, guardWindowClose: guardWindowClose);
      await toReview(tester);
      if (dry) {
        await tap(tester, find.text('Advanced'));
        await tap(tester, key('dry-run'));
      }
      await tap(tester, key('install-button'));
      await tester.tap(key('confirm-install-yes'));
      await pumpFor(tester);
      return engine.flashes.single;
    }

    testWidgets('during the checks, Stop stops at once', (tester) async {
      final engine = ControlledEngine()..holdFlash = true;
      final run = await startInstall(tester, engine);
      run.add(const StageEvent('check', 'Checking the image'));
      await pumpFor(tester);
      await tapRunning(tester, key('stop-install'));
      expect(key('confirm-stop'), findsNothing);
      expect(run.cancelled, isTrue);
      await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
      await tester.pumpAndSettle();
    });

    testWidgets('once writing, Stop asks first', (tester) async {
      final engine = ControlledEngine()..holdFlash = true;
      final run = await startInstall(tester, engine);
      run.add(const StageEvent('check', 'Checking the image'));
      run.add(const StageEvent('flash', 'Writing Thimsy'));
      await pumpFor(tester);

      await tapRunning(tester, key('stop-install'));
      expect(key('confirm-stop'), findsOneWidget);
      expect(find.textContaining('recovered from download mode'), findsOne);
      await tapRunning(tester, key('confirm-stop-keep'));
      expect(key('confirm-stop'), findsNothing);
      expect(run.cancelled, isFalse);

      await tapRunning(tester, key('stop-install'));
      await tapRunning(tester, key('confirm-stop-yes'));
      expect(run.cancelled, isTrue);
      await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
      await tester.pumpAndSettle();
    });

    testWidgets('a dry run stops without asking', (tester) async {
      final engine = ControlledEngine()..holdFlash = true;
      final run = await startInstall(tester, engine, dry: true);
      expect(engine.calls, contains('flash update dry'));
      run.add(const StageEvent('check', 'Checking the image'));
      run.add(const StageEvent('flash', 'Writing Thimsy'));
      await pumpFor(tester);
      await tapRunning(tester, key('stop-install'));
      expect(key('confirm-stop'), findsNothing);
      expect(run.cancelled, isTrue);
      await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
      await tester.pumpAndSettle();
    });

    testWidgets('closing the window while writing asks the same', (
      tester,
    ) async {
      final engine = ControlledEngine()..holdFlash = true;
      // Guarded explicitly: the default depends on the platform (none on
      // macOS, where CI also runs this).
      final run = await startInstall(tester, engine, guardWindowClose: true);
      run.add(const StageEvent('check', 'Checking the image'));
      run.add(const StageEvent('flash', 'Writing Thimsy'));
      await pumpFor(tester);

      final keep = tester.binding.handleRequestAppExit();
      await pumpFor(tester);
      expect(key('confirm-stop'), findsOneWidget);
      await tapRunning(tester, key('confirm-stop-keep'));
      expect(await keep, AppExitResponse.cancel);
      expect(run.cancelled, isFalse);

      final stop = tester.binding.handleRequestAppExit();
      await pumpFor(tester);
      await tapRunning(tester, key('confirm-stop-yes'));
      expect(await stop, AppExitResponse.exit);
      expect(run.cancelled, isTrue);
      await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
      await tester.pumpAndSettle();
    });

    testWidgets('closing the window during the checks stops the install', (
      tester,
    ) async {
      final engine = ControlledEngine()..holdFlash = true;
      final s = await startApp(tester, engine, guardWindowClose: true);
      await toReview(tester);
      await tap(tester, key('install-button'));
      await tester.tap(key('confirm-install-yes'));
      await pumpFor(tester);
      final run = engine.flashes.single;
      run.add(const StageEvent('check', 'Checking the image'));
      await pumpFor(tester);
      expect(s.stopNeedsConfirmation, isFalse);

      final exit = tester.binding.handleRequestAppExit();
      await pumpFor(tester);
      expect(key('confirm-stop'), findsNothing);
      expect(await exit, AppExitResponse.exit);
      // Not left running behind the closed window, able to start writing.
      expect(run.cancelled, isTrue);
      await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
      await tester.pumpAndSettle();
    });

    testWidgets('closing the window stops a running build', (tester) async {
      final engine = ControlledEngine()..holdBuild = true;
      await startApp(tester, engine, guardWindowClose: true);
      await tap(tester, key('device-$cube'));
      await next(tester);
      await next(tester);
      await tap(tester, key('journey-first_time'));
      await next(tester);
      await tester.enterText(key('workspace-field'), '/tmp/thimsy-ws');
      await tester.pumpAndSettle();
      await tapRunning(tester, key('start-build'));
      final run = engine.builds.single;

      expect(await tester.binding.handleRequestAppExit(), AppExitResponse.exit);
      expect(run.cancelled, isTrue);
      await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
      await tester.pumpAndSettle();
    });

    testWidgets(
      'macOS default: closing doesn\'t ask, but still stops the install',
      (tester) async {
        expect(HomeShell.defaultGuardWindowClose, isFalse);
        final engine = ControlledEngine()..holdFlash = true;
        // No guardWindowClose: the platform default.
        final run = await startInstall(tester, engine);
        run.add(const StageEvent('check', 'Checking the image'));
        run.add(const StageEvent('flash', 'Writing Thimsy'));
        await pumpFor(tester);

        final exit = tester.binding.handleRequestAppExit();
        await pumpFor(tester);
        expect(key('confirm-stop'), findsNothing);
        expect(await exit, AppExitResponse.exit);
        expect(run.cancelled, isTrue);
        await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
        await tester.pumpAndSettle();
      },
      variant: TargetPlatformVariant.only(TargetPlatform.macOS),
    );

    test('the window close is guarded by default on Windows and Linux', () {
      try {
        for (final (platform, guarded) in [
          (TargetPlatform.windows, true),
          (TargetPlatform.linux, true),
          (TargetPlatform.macOS, false),
        ]) {
          debugDefaultTargetPlatformOverride = platform;
          expect(
            HomeShell.defaultGuardWindowClose,
            guarded,
            reason: '$platform',
          );
        }
      } finally {
        debugDefaultTargetPlatformOverride = null;
      }
    });

    testWidgets('the engines are stopped when the window goes away', (
      tester,
    ) async {
      // macOS may quit without asking: the shell's dispose still stops them.
      final engine = ControlledEngine()..holdFlash = true;
      final run = await startInstall(tester, engine);
      run.add(const StageEvent('check', 'Checking the image'));
      run.add(const StageEvent('flash', 'Writing Thimsy'));
      await pumpFor(tester);
      await tester.pumpWidget(const SizedBox());
      expect(run.cancelled, isTrue);
      await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
    });
  });

  group('a dry run is never reported as installed', () {
    testWidgets('after a dry run, Next stays off on Install', (tester) async {
      final engine = ControlledEngine();
      final s = await startApp(tester, engine);
      await toReview(tester);
      await tap(tester, find.text('Advanced'));
      await tap(tester, key('dry-run'));
      await tap(tester, key('install-button'));
      await tap(tester, key('confirm-install-yes'));
      expect(engine.calls, contains('flash update dry'));
      expect(s.flashRun.ok, isTrue);
      expect(s.canProceed(WizardStep.install), isFalse);
      expect(tester.widget<FilledButton>(key('next')).onPressed, isNull);
      s.next();
      expect(s.step, WizardStep.install);
      expect(find.text('Thimsy is installed'), findsNothing);
    });
  });

  group('the device watch', () {
    Future<(ControlledEngine, AppState)> watching() async {
      final engine = ControlledEngine()..holdDetect = true;
      final s = await toBuild(engine);
      await s.startBuild();
      await settle();
      s.next(); // backup
      s.setBackedUp(true);
      s.next(); // unlock
      s.setUnlockConfirmed(true);
      s.next(); // connect: the watch starts
      expect(s.step, WizardStep.connect);
      engine.detects.single.add(
        const DevicesEvent([
          DetectedDevice(
            vidpid: '1782:4d00',
            mode: 'download',
            device: cube,
            label: 'Unisoc download mode',
          ),
        ]),
      );
      await settle();
      expect(s.matchedDevice, isNotNull);
      return (engine, s);
    }

    test(
      'a watch that ends forgets its devices and can be restarted',
      () async {
        final (engine, s) = await watching();
        await engine.detects.single.finish();
        await settle();
        expect(s.detected, isEmpty);
        expect(s.matchedDevice, isNull);
        expect(s.detectError?.code, 'detect_stopped');
        expect(s.canProceed(WizardStep.connect), isFalse);

        s.restartDetect();
        expect(engine.detects, hasLength(2));
        expect(s.detectError, isNull);
        engine.detects.last.add(const DevicesEvent([]));
        await settle();
        expect(s.matchedDevice, isNull);
      },
    );

    test('a watch that reports an error is stopped and shows it', () async {
      final (engine, s) = await watching();
      final first = engine.detects.single;
      first.add(
        const ErrorEvent(code: 'usb_error', message: 'USB access denied'),
      );
      await settle();
      expect(first.cancelled, isTrue);
      expect(s.detected, isEmpty);
      expect(s.detectError?.message, 'USB access denied');
      // Its late lines change nothing.
      first.add(
        const DevicesEvent([
          DetectedDevice(
            vidpid: '1782:4d00',
            mode: 'download',
            device: cube,
            label: 'late',
          ),
        ]),
      );
      await first.finish();
      await settle();
      expect(s.detected, isEmpty);
      expect(s.detectError?.message, 'USB access denied');
      // Try again starts a new watch.
      s.restartDetect();
      expect(engine.detects, hasLength(2));
    });

    testWidgets('Connect shows the stopped watch with Try again', (
      tester,
    ) async {
      final engine = ControlledEngine()..holdDetect = true;
      await startApp(tester, engine);
      await tap(tester, key('device-$cube'));
      await next(tester);
      await next(tester);
      await tap(tester, key('journey-returning'));
      await next(tester);
      await tester.enterText(key('workspace-field'), '/tmp/thimsy-ws');
      await tester.pumpAndSettle();
      await tap(tester, key('start-build'));
      await next(tester);
      await tap(tester, key('backed-up'));
      await next(tester);
      expect(engine.detects, hasLength(1));
      await engine.detects.single.finish();
      await tester.pumpAndSettle();
      expect(key('detect-error'), findsOneWidget);
      expect(find.text('Stopped looking for the device.'), findsOneWidget);
      await tap(tester, key('detect-retry'));
      expect(engine.detects, hasLength(2));
      expect(key('detect-error'), findsNothing);
    });
  });

  group('an unlock run locks the wizard', () {
    test('no selection or step change while unlocking', () async {
      final engine = ControlledEngine()..holdUnlock = true;
      final s = await toBuild(engine, device: rp4);
      expect(s.unlockRoute, 'automated');
      s.startUnlock();
      expect(s.busy, isTrue);
      await s.selectDevice(cube);
      expect(s.deviceId, rp4);
      s.goTo(WizardStep.device);
      expect(s.step, WizardStep.build);
      expect(s.canGoTo(WizardStep.profile), isFalse);

      final run = engine.unlocks.single;
      s.cancelUnlock();
      expect(run.cancelled, isTrue);
      await run.finish(ResultEvent(false, {'type': 'result', 'ok': false}));
      await settle();
      expect(s.busy, isFalse);
      expect(s.canGoTo(WizardStep.profile), isTrue);
    });
  });

  test('a language change keeps the plans until the new ones are in', () async {
    final engine = ControlledEngine();
    final s = AppState(engine, onAndroid: false, workspace: '/ws');
    await s.loadCatalog();
    await s.selectDevice(cube);
    s.deviceNext();
    await settle();
    expect(s.plans, hasLength(3));
    expect(s.profileId, 'medium');

    final gate = engine.resolveGates[cube] = Completer<void>();
    engine.resolveLanguages.clear();
    s.useEffectiveLocale(const Locale('de'));
    await until(() => s.plansLoading);
    expect(s.plansLoading, isTrue);
    // Still all there while the German ones load.
    expect(s.plans, hasLength(3));
    expect(s.plan, isNotNull);
    // Another change meanwhile: queued, not mixed in.
    s.useEffectiveLocale(const Locale('fr'));
    await settle();
    expect(s.plans, hasLength(3));
    gate.complete();
    await until(() => engine.resolveLanguages.length == 6 && !s.plansLoading);
    expect(s.plansLoading, isFalse);
    expect(s.plans, hasLength(3));
    expect(s.plan, isNotNull);
    // The last pass was made in the last language.
    expect(engine.resolveLanguages, hasLength(6));
    expect(engine.resolveLanguages.last, 'fr');
    expect(engine.calls.where((c) => c == 'catalog'), hasLength(3));
  });

  test('a driver check answer for another device is dropped', () async {
    final engine = ControlledEngine(host: 'windows');
    final s = AppState(engine, onAndroid: false, workspace: '/ws');
    await s.loadCatalog();
    await s.selectDevice(cube);
    engine.driverGate = Completer<void>();
    final checking = s.checkDriver();
    expect(s.driverBusy, isTrue);
    await s.selectDevice(rp4);
    expect(s.driverBusy, isFalse);
    engine.driverGate!.complete();
    await checking;
    expect(s.driverStatus, isNull);
    expect(s.driverDetail, isNull);
    expect(s.driverBusy, isFalse);
    // A check for the device chosen now still lands.
    await s.checkDriver();
    expect(s.driverStatus, 'missing');
  });

  test('plans resolved for an earlier device are dropped', () async {
    final engine = ControlledEngine();
    final s = AppState(engine, onAndroid: false, workspace: '/ws');
    await s.loadCatalog();
    final gateA = engine.resolveGates[cube] = Completer<void>();
    final gateB = engine.resolveGates[rp4] = Completer<void>();
    await s.selectDevice(cube);
    s.deviceNext(); // enters Profile: resolves for A start (held)
    await settle();
    s.goTo(WizardStep.device);
    await s.selectDevice(rp4);
    s.deviceNext(); // resolves for B
    await settle();
    gateB.complete();
    await settle();
    gateA.complete(); // A's answers come in last
    await settle();
    expect(s.plansLoading, isFalse);
    expect(s.plans, hasLength(3));
    expect(s.plans.values.map((p) => p.device).toSet(), {rp4});
    expect(s.plan?.device, rp4);
  });

  test('Review never relies on an old detected-device list', () async {
    final engine = ControlledEngine();
    final s = await toBuild(engine);
    await s.startBuild();
    await settle();
    s.next(); // backup
    s.setBackedUp(true);
    s.next(); // unlock
    s.setUnlockConfirmed(true);
    s.next(); // connect: the watch starts
    expect(s.step, WizardStep.connect);
    await settle();
    expect(s.matchedDevice, isNotNull);
    s.next(); // review: still watching
    expect(s.step, WizardStep.review);
    expect(engine.calls.where((c) => c == 'detect'), hasLength(1));
    expect(s.matchedDevice, isNotNull);

    // Leaving for an earlier step stops the watch and forgets the list.
    s.goTo(WizardStep.backup);
    expect(s.detected, isEmpty);
    expect(s.matchedDevice, isNull);
    s.goTo(WizardStep.review); // watching again
    await settle();
    expect(s.matchedDevice, isNotNull);
    expect(engine.calls.where((c) => c == 'detect'), hasLength(2));

    s.goTo(WizardStep.device);
    await s.selectDevice(rp4);
    expect(s.detected, isEmpty);
  });
}
