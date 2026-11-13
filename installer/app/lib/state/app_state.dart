import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart' show Locale;

import '../engine/engine.dart';
import '../l10n/languages.dart';

/// Where the chosen language is remembered (shared_preferences in the app).
abstract class LanguageStore {
  String? load();
  Future<void> save(String? code);
}

/// The walkthrough steps, in order.
enum WizardStep {
  device,
  profile,
  journey,
  build,
  backup,
  unlock,
  connect,
  review,
  install,
  firstBoot,
  done,
}

/// Progress of one streaming engine command (build, unlock, flash).
class RunState {
  bool running = false;
  String? stage;
  String? stageTitle;
  double? fraction;
  String? label;
  PromptEvent? prompt;
  ErrorEvent? error;
  ResultEvent? result;
  final List<String> log = [];

  bool get ok => result?.ok == true;
  bool get failed => result != null && !result!.ok;

  void reset() {
    running = false;
    stage = stageTitle = label = null;
    fraction = null;
    prompt = null;
    error = null;
    result = null;
    log.clear();
  }

  /// Folds one event into the state.
  void apply(EngineEvent e) {
    switch (e) {
      case StageEvent():
        stage = e.stage;
        stageTitle = e.title;
        fraction = null;
        label = null;
        prompt = null;
        log.add('— ${e.title}');
      case ProgressEvent():
        fraction = e.fraction;
        label = e.label;
      case LogEvent():
        log.add(e.message);
      case PromptEvent():
        prompt = e;
      case ErrorEvent():
        error = e;
        log.add('error ${e.code}: ${e.message}');
      case ResultEvent():
        result = e;
        running = false;
        prompt = null;
      default:
        break;
    }
  }
}

/// Where a not-enough-space problem came from.
enum SpaceProblemKind { build, flash }

class SpaceProblem {
  final SpaceProblemKind kind;

  /// Bytes needed / available (build check), when known.
  final int need;
  final int free;

  /// The engine's own message (flash), when given.
  final String? message;
  const SpaceProblem(this.kind, {this.need = 0, this.free = 0, this.message});
}

/// The drive part of [path] for messages: `C:` on Windows, else the path.
String driveOf(String path) {
  final m = RegExp(r'^([A-Za-z]:)').firstMatch(path);
  return m != null ? m.group(1)!.toUpperCase() : path;
}

/// Where the build gets the Thimsy device pack.
enum ImageSource {
  /// The engine downloads it (the default).
  download,

  /// Files the user already downloaded: one `.tar`, or its numbered parts.
  files,
}

/// What the device step's Next did.
enum DeviceNext { proceeded, needsWindows, nothingSelected }

class AppState extends ChangeNotifier {
  final Engine engine;

  /// True when running on Android, where the engine isn't bundled yet.
  final bool onAndroid;

  final LanguageStore? languageStore;

  /// [workspace] is the starting workspace folder (see
  /// `findDefaultWorkspace`); the user can change it on the build step.
  AppState(
    this.engine, {
    bool? onAndroid,
    this.languageStore,
    this.workspace = '',
  }) : onAndroid = onAndroid ?? (!kIsWeb && Platform.isAndroid) {
    languageCode = languageByCode(languageStore?.load())?.code;
  }

  // -------------------------------------------------------------- language
  /// Chosen Thimsy language code, or null to follow the system.
  String? languageCode;

  /// The locale MaterialApp should use (null = system).
  Locale? get locale => languageByCode(languageCode)?.locale;

  void setLanguage(String? code) {
    languageCode = languageByCode(code)?.code;
    languageStore?.save(languageCode);
    notifyListeners();
  }

  /// Called with the locale MaterialApp actually resolved. Passes it to the
  /// engine as `--lang` and re-fetches engine text in the new language.
  void useEffectiveLocale(Locale l) {
    final code = languageForLocale(l).code;
    if (engine.language == code) return;
    engine.language = code;
    if (catalog != null) scheduleMicrotask(_reloadTranslated);
  }

  Future<void> _reloadTranslated() async {
    await loadCatalog();
    if (plans.isNotEmpty) {
      plans.clear();
      await loadPlans();
    }
  }

  // ------------------------------------------------------------ navigation
  WizardStep step = WizardStep.device;
  WizardStep furthest = WizardStep.device;

  // --------------------------------------------------------------- catalog
  Catalog? catalog;

  /// Why the catalog failed to load (an [EngineException] or other error).
  Object? catalogError;
  bool worksHereOnly = false;
  String? deviceId;

  String get host => catalog?.host ?? 'linux';

  DeviceInfo? get device =>
      catalog?.devices.where((d) => d.id == deviceId).firstOrNull;

  List<DeviceInfo> get visibleDevices => [
    for (final d in catalog?.devices ?? const <DeviceInfo>[])
      if (!worksHereOnly || d.worksOn(host)) d,
  ];

  /// Build and export only (this computer can't flash the device).
  bool exportOnly = false;

  // -------------------------------------------------------------- profiles
  final Map<String, Plan> plans = {};
  Object? planError;
  bool plansLoading = false;
  String? profileId;
  Plan? get plan => profileId == null ? null : plans[profileId];

  // --------------------------------------------------------------- journey
  /// `first_time` or `returning` (see [Journey]).
  String? journeyId;
  Journey? get journey => journeyId == null ? null : plan?.journeys[journeyId];

  /// Modes Review offers for the chosen journey, default first.
  List<FlashMode> get allowedModes {
    final modes = journey?.modes ?? const ['full'];
    final out = [
      for (final m in modes)
        if (m == 'update')
          FlashMode.update
        else if (m == 'full')
          FlashMode.full,
    ];
    return out.isEmpty ? const [FlashMode.full] : out;
  }

  // ----------------------------------------------------------------- build
  String workspace;
  ImageSource imageSource = ImageSource.download;

  /// The device pack files chosen for [ImageSource.files], sorted so
  /// numbered parts are in order.
  List<String> importFiles = const [];

  /// What the build is given as `--import`: nothing when downloading.
  List<String> get imports =>
      imageSource == ImageSource.files ? importFiles : const [];

  /// False while "Use a file I already downloaded" has no file yet.
  bool get imageSourceReady =>
      imageSource == ImageSource.download || importFiles.isNotEmpty;
  bool deleteDownloads = false;
  SpaceInfo? space;
  bool spaceChecking = false;
  Object? spaceError;
  Timer? _spaceDebounce;

  /// Set when there isn't enough room; the shell shows the dialog.
  SpaceProblem? spaceProblem;

  final RunState buildRun = RunState();
  EngineRun? _buildHandle;
  String? get job => buildRun.result?.str('job');
  String? get image => buildRun.result?.str('image');

  // ---------------------------------------------------------------- backup
  bool backedUp = false;

  // ---------------------------------------------------------------- unlock
  bool unlockConfirmed = false;
  final RunState unlockRun = RunState();
  String get unlockRoute => plan?.guides.unlockRoute ?? 'none';

  // --------------------------------------------------------------- connect
  List<DetectedDevice> detected = const [];
  EngineRun? _detectHandle;
  StreamSubscription<EngineEvent>? _detectSub;
  String? driverStatus;
  String? driverDetail;
  bool driverBusy = false;

  DetectedDevice? get matchedDevice =>
      detected.where((d) => d.device == deviceId).firstOrNull;

  // --------------------------------------------------------------- install
  bool dryRun = false;

  /// The mode the current or last flash run used. Fixed when it starts, so
  /// the install screen never reports a dry run as a real install (or the
  /// reverse), and "Run again" repeats the same mode.
  bool flashRunDry = false;
  FlashMode flashMode = FlashMode.full;
  final RunState flashRun = RunState();
  EngineRun? _flashHandle;

  // ----------------------------------------------------------------- clean
  List<CleanItem>? cleanItems;
  final Map<String, bool> cleanChoice = {
    'scratch': true,
    'downloads': true,
    'image': false,
    'backup': false,
  };
  bool cleanBusy = false;
  int? freedBytes;

  // ------------------------------------------------------------ first boot
  final Set<int> firstBootChecks = {};

  /// Set when the engine says this computer can't install the device; the
  /// shell shows the needs-Windows dialog and clears it.
  ErrorEvent? hostError;

  // ============================================================== actions

  Future<void> loadCatalog() async {
    catalogError = null;
    notifyListeners();
    try {
      catalog = await engine.catalog();
    } catch (e) {
      catalogError = e;
    }
    notifyListeners();
  }

  void setWorksHereOnly(bool v) {
    worksHereOnly = v;
    notifyListeners();
  }

  void selectDevice(String id) {
    if (deviceId != id) {
      deviceId = id;
      plans.clear();
      profileId = null;
      exportOnly = false;
      _resetFrom(WizardStep.profile);
    }
    notifyListeners();
  }

  /// Next on the device step. Returns [DeviceNext.needsWindows] when this
  /// computer can't install the device; the UI then offers export.
  DeviceNext deviceNext() {
    final d = device;
    if (d == null) return DeviceNext.nothingSelected;
    if (!d.installableOn(host)) return DeviceNext.needsWindows;
    exportOnly = false;
    _advanceTo(WizardStep.profile);
    return DeviceNext.proceeded;
  }

  /// "Build and export anyway" from the needs-Windows dialog.
  void continueExportOnly() {
    if (device?.buildVerified != true) return;
    exportOnly = true;
    _advanceTo(WizardStep.profile);
  }

  /// "Build and export anyway" from an engine `unsupported_host` error:
  /// with an image already built, go straight to Done; otherwise continue
  /// to the build in export-only mode.
  void exportAnyway() {
    if (device?.buildVerified != true) return;
    exportOnly = true;
    if (buildRun.ok) {
      _advanceTo(WizardStep.done);
    } else if (step.index < WizardStep.build.index) {
      _advanceTo(WizardStep.profile);
    } else {
      _advanceTo(WizardStep.build);
    }
  }

  Future<void> loadPlans() async {
    final d = deviceId;
    final cat = catalog;
    if (d == null || cat == null) return;
    plansLoading = true;
    planError = null;
    notifyListeners();
    try {
      for (final p in cat.profiles) {
        plans[p.id] = await engine.resolve(device: d, profile: p.id);
      }
      profileId ??= cat.profiles
          .where((p) => p.available && (plans[p.id]?.available ?? false))
          .map((p) => p.id)
          .firstOrNull;
    } catch (e) {
      planError = e;
    } finally {
      plansLoading = false;
      notifyListeners();
    }
  }

  void selectProfile(String id) {
    if (!(plans[id]?.available ?? false)) return;
    if (profileId != id) {
      profileId = id;
      _resetFrom(WizardStep.build);
    }
    notifyListeners();
  }

  void selectJourney(String id) {
    if (plan?.journeys[id] == null) return;
    journeyId = id;
    flashMode = allowedModes.first;
    unlockConfirmed = false;
    notifyListeners();
  }

  void setWorkspace(String v) {
    workspace = v;
    space = null;
    notifyListeners();
    _spaceDebounce?.cancel();
    if (v.trim().isEmpty) return;
    _spaceDebounce = Timer(
      const Duration(milliseconds: 400),
      () => checkSpace(),
    );
  }

  void setDeleteDownloads(bool v) {
    deleteDownloads = v;
    notifyListeners();
    if (workspace.trim().isNotEmpty) checkSpace();
  }

  /// Runs `space` for the current workspace. Returns null on failure.
  Future<SpaceInfo?> checkSpace() async {
    final d = deviceId, p = profileId, w = workspace.trim();
    if (d == null || p == null || w.isEmpty) return null;
    spaceChecking = true;
    spaceError = null;
    notifyListeners();
    try {
      final info = await engine.space(
        device: d,
        profile: p,
        workspace: w,
        deleteDownloads: deleteDownloads,
      );
      if (workspace.trim() == w) space = info;
      return info;
    } catch (e) {
      spaceError = e;
      return null;
    } finally {
      spaceChecking = false;
      notifyListeners();
    }
  }

  void clearSpaceProblem() {
    spaceProblem = null;
    notifyListeners();
  }

  /// The dialog's "Check again": re-runs the check and carries on (build or
  /// flash) when there's room now; otherwise the problem comes back.
  Future<void> recheckSpace() async {
    final problem = spaceProblem;
    spaceProblem = null;
    notifyListeners();
    if (problem?.kind == SpaceProblemKind.flash) {
      startFlash();
    } else {
      await startBuild();
    }
  }

  /// Switches back to downloading. (Choosing files goes through
  /// [setImportFiles].)
  void useDownload() {
    imageSource = ImageSource.download;
    notifyListeners();
  }

  /// Uses [files] (one `.tar`, or its numbered parts) instead of
  /// downloading. An empty list keeps the current choice.
  void setImportFiles(List<String> files) {
    if (files.isEmpty) return;
    importFiles = List.unmodifiable([...files]..sort());
    imageSource = ImageSource.files;
    notifyListeners();
  }

  /// Checks free space first; never starts the build without room.
  Future<void> startBuild() async {
    if (buildRun.running || deviceId == null || profileId == null) return;
    if (workspace.trim().isEmpty || !imageSourceReady) return;
    _spaceDebounce?.cancel();
    final info = await checkSpace();
    if (info == null) return;
    if (!info.ok) {
      spaceProblem = SpaceProblem(
        SpaceProblemKind.build,
        need: info.need,
        free: info.free,
      );
      notifyListeners();
      return;
    }
    buildRun.reset();
    buildRun.running = true;
    notifyListeners();
    final run = engine.build(
      device: deviceId!,
      profile: profileId!,
      workspace: workspace.trim(),
      imports: imports,
      deleteDownloads: deleteDownloads,
    );
    _buildHandle = run;
    _follow(run, buildRun, onDone: () => _buildHandle = null);
  }

  Future<void> cancelBuild() async {
    await _buildHandle?.cancel();
  }

  void setBackedUp(bool v) {
    backedUp = v;
    notifyListeners();
  }

  void setUnlockConfirmed(bool v) {
    unlockConfirmed = v;
    notifyListeners();
  }

  void startUnlock() {
    if (unlockRun.running || deviceId == null) return;
    unlockRun.reset();
    unlockRun.running = true;
    notifyListeners();
    _follow(engine.unlock(device: deviceId!), unlockRun);
  }

  void startDetect() {
    if (_detectHandle != null) return;
    final run = engine.detect();
    _detectHandle = run;
    _detectSub = run.events.listen((e) {
      if (e is DevicesEvent) {
        detected = e.devices;
        notifyListeners();
      }
    });
    if (host == 'windows' && deviceId != null) checkDriver();
  }

  Future<void> stopDetect() async {
    final h = _detectHandle;
    _detectHandle = null;
    await _detectSub?.cancel();
    _detectSub = null;
    await h?.cancel();
  }

  Future<void> checkDriver({bool install = false}) async {
    if (deviceId == null) return;
    driverBusy = true;
    notifyListeners();
    try {
      final r = await engine.drivers(device: deviceId!, install: install);
      driverStatus = r.str('status');
      driverDetail = r.str('detail');
    } on EngineException catch (e) {
      driverStatus = 'missing';
      driverDetail = e.event.hint ?? e.event.message;
    } finally {
      driverBusy = false;
      notifyListeners();
    }
  }

  void setDryRun(bool v) {
    dryRun = v;
    notifyListeners();
  }

  void setFlashMode(FlashMode m) {
    if (!allowedModes.contains(m)) return;
    flashMode = m;
    notifyListeners();
  }

  /// Whether Review may start a real install: devices that must already be
  /// connected (not Unisoc, whose tool waits) need to have been detected.
  bool get canInstallForReal =>
      plan?.connectFirst == false || matchedDevice != null;

  /// Repeats the last run in the same mode (never switches a dry run to a
  /// real install: that only happens from Review, after its confirmation).
  void runAgain() {
    dryRun = flashRunDry;
    startFlash();
  }

  /// Review's Install button (after the confirmation dialog).
  void install() {
    _advanceTo(WizardStep.install);
    startFlash();
  }

  void startFlash() {
    final j = job;
    if (flashRun.running || j == null) return;
    flashRun.reset();
    flashRun.running = true;
    flashRunDry = dryRun;
    notifyListeners();
    final run = engine.flash(job: j, dryRun: flashRunDry, mode: flashMode);
    _flashHandle = run;
    _follow(run, flashRun, onDone: () => _flashHandle = null);
  }

  Future<void> cancelFlash() async {
    await _flashHandle?.cancel();
  }

  Future<void> loadCleanItems() async {
    if (workspace.trim().isEmpty) return;
    try {
      final r = await engine.clean(workspace: workspace.trim(), dryRun: true);
      cleanItems = CleanItem.listFrom(r);
    } on EngineException {
      cleanItems = const [];
    }
    notifyListeners();
  }

  void setCleanChoice(String kind, bool v) {
    cleanChoice[kind] = v;
    notifyListeners();
  }

  Future<void> runClean() async {
    if (cleanBusy || workspace.trim().isEmpty) return;
    cleanBusy = true;
    notifyListeners();
    try {
      final r = await engine.clean(
        workspace: workspace.trim(),
        downloads: cleanChoice['downloads'] ?? false,
        image: cleanChoice['image'] ?? false,
        scratch: cleanChoice['scratch'] ?? false,
        backup: cleanChoice['backup'] ?? false,
      );
      final freed = r.data['freed'];
      freedBytes = freed is num ? freed.toInt() : 0;
    } on EngineException {
      freedBytes = 0;
    } finally {
      cleanBusy = false;
      notifyListeners();
    }
    await loadCleanItems();
  }

  void toggleFirstBoot(int i) {
    firstBootChecks.contains(i)
        ? firstBootChecks.remove(i)
        : firstBootChecks.add(i);
    notifyListeners();
  }

  void clearHostError() {
    hostError = null;
    notifyListeners();
  }

  void _follow(EngineRun run, RunState state, {VoidCallback? onDone}) {
    run.events.listen(
      (e) {
        state.apply(e);
        if (e is ErrorEvent && e.code == 'unsupported_host') hostError = e;
        if (e is ErrorEvent && e.code == 'not_enough_space') {
          spaceProblem = SpaceProblem(
            identical(state, flashRun)
                ? SpaceProblemKind.flash
                : SpaceProblemKind.build,
            message: e.message,
          );
        }
        notifyListeners();
      },
      onDone: () {
        state.running = false;
        onDone?.call();
        notifyListeners();
      },
    );
  }

  // ----------------------------------------------------------- navigation

  /// Steps skipped when only building and exporting.
  static const _flashSteps = {
    WizardStep.backup,
    WizardStep.unlock,
    WizardStep.connect,
    WizardStep.review,
    WizardStep.install,
    WizardStep.firstBoot,
  };

  bool isSkipped(WizardStep s) =>
      (exportOnly && (_flashSteps.contains(s) || s == WizardStep.journey)) ||
      (s == WizardStep.unlock &&
          ((unlockRoute == 'none' && plan != null) ||
              journey?.unlock == false));

  bool canProceed(WizardStep s) {
    switch (s) {
      case WizardStep.device:
        return device != null;
      case WizardStep.profile:
        return plan?.available ?? false;
      case WizardStep.journey:
        return journey != null;
      case WizardStep.build:
        return buildRun.ok;
      case WizardStep.backup:
        return backedUp;
      case WizardStep.unlock:
        return switch (unlockRoute) {
          'automated' => unlockRun.ok || unlockConfirmed,
          'guided' => unlockConfirmed,
          _ => true,
        };
      case WizardStep.connect:
        // Unisoc: the flash tool waits for the device, so connecting comes
        // after installing starts (download mode lasts only seconds otherwise).
        return matchedDevice != null || dryRun || plan?.connectFirst == false;
      case WizardStep.review:
        return false; // Install button instead
      case WizardStep.install:
        return flashRun.ok;
      case WizardStep.firstBoot:
        return true;
      case WizardStep.done:
        return false;
    }
  }

  /// Generic Next (the device step uses [deviceNext]).
  void next() {
    if (!canProceed(step)) return;
    var i = step.index + 1;
    while (i < WizardStep.values.length && isSkipped(WizardStep.values[i])) {
      i++;
    }
    if (i < WizardStep.values.length) _advanceTo(WizardStep.values[i]);
  }

  void back() {
    var i = step.index - 1;
    while (i > 0 && isSkipped(WizardStep.values[i])) {
      i--;
    }
    if (i >= 0) goTo(WizardStep.values[i]);
  }

  bool canGoTo(WizardStep s) => s.index <= furthest.index && !isSkipped(s);

  void goTo(WizardStep s) {
    if (!canGoTo(s)) return;
    _enter(s);
  }

  void _advanceTo(WizardStep s) {
    if (s.index > furthest.index) furthest = s;
    _enter(s);
  }

  void _enter(WizardStep s) {
    final leaving = step;
    step = s;
    if (leaving == WizardStep.connect && s != WizardStep.connect) stopDetect();
    if (s == WizardStep.connect) startDetect();
    if (s == WizardStep.profile && plans.isEmpty) loadPlans();
    if (s == WizardStep.build && space == null && workspace.isNotEmpty) {
      checkSpace();
    }
    if (s == WizardStep.done) loadCleanItems();
    notifyListeners();
  }

  /// Forget progress from [s] on (a different device or profile was picked).
  void _resetFrom(WizardStep s) {
    if (furthest.index >= s.index) furthest = WizardStep.values[s.index - 1];
    if (s.index <= WizardStep.profile.index) journeyId = null;
    if (s.index <= WizardStep.build.index) buildRun.reset();
    backedUp = false;
    unlockConfirmed = false;
    unlockRun.reset();
    flashRun.reset();
    firstBootChecks.clear();
  }

  @override
  void dispose() {
    _spaceDebounce?.cancel();
    stopDetect();
    _buildHandle?.cancel();
    _flashHandle?.cancel();
    super.dispose();
  }
}
