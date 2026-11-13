// Guards against using a stale build, repeating an unconfirmed install,
// stopping a real install by accident, and mixing plans or detected devices
// from an earlier selection.
import 'dart:async';
import 'dart:ui' show AppExitResponse;

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:thimsy_installer/engine/engine.dart';
import 'package:thimsy_installer/engine/fake_engine.dart';
import 'package:thimsy_installer/main.dart';
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
  ControlledEngine() : super(step: Duration.zero);

  bool holdBuild = false;
  bool holdFlash = false;
  final List<HeldRun> builds = [];
  final List<HeldRun> flashes = [];

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
    await resolveGates[device]?.future;
    return super.resolve(device: device, profile: profile);
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

// ------------------------------------------------------------ widget helpers

Future<AppState> startApp(WidgetTester tester, FakeEngine engine) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final state = AppState(engine, onAndroid: false);
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

    test('Check again after a flash space error keeps the dry run', () async {
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
      // Dry run switched off afterwards: the retry is still the dry run.
      s.setDryRun(false);
      await s.recheckSpace();
      await settle();
      expect(engine.calls.where((c) => c.startsWith('flash')), [
        'flash full dry',
        'flash full dry',
      ]);
      expect(s.flashRun.ok, isTrue);
      expect(s.flashRunDry, isTrue);
    });
  });

  group('Stop during a real install', () {
    Future<HeldRun> startInstall(
      WidgetTester tester,
      ControlledEngine engine, {
      bool dry = false,
    }) async {
      await startApp(tester, engine);
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
      final run = await startInstall(tester, engine);
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
