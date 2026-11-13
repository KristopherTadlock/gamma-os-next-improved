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

  /// Bumped by [reset]: events from a run started before the reset (one
  /// that was replaced or abandoned) are ignored.
  int generation = 0;

  bool get ok => result?.ok == true;
  bool get failed => result != null && !result!.ok;

  void reset() {
    generation++;
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

  /// What to tell the user (flash): the engine's hint (translated), else
  /// its message.
  final String? message;

  /// The engine's own (English) message, shown under "Details".
  final String? details;

  /// The space check's `reason` (`not_enough_space`, `fat32`,
  /// `probe_failed`), when it gave one.
  final String? reason;
  const SpaceProblem(
    this.kind, {
    this.need = 0,
    this.free = 0,
    this.message,
    this.details,
    this.reason,
  });
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

/// What the user confirmed on Review: the only flash "Run again" (or the
/// space dialog's "Check again") may repeat.
class FlashConfirmation {
  final String job;
  final FlashMode mode;
  final bool dry;
  const FlashConfirmation(this.job, this.mode, this.dry);
}

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

  /// A translated reload is running; another language change meanwhile
  /// queues one more pass (so the last language always wins).
  bool _reloading = false, _reloadQueued = false;

  /// Fetches the catalog and the plans again in the engine's language. The
  /// plans shown stay until the new ones are all in ([loadPlans] swaps
  /// them), so the profile step never empties mid-flow.
  Future<void> _reloadTranslated() async {
    if (_reloading) {
      _reloadQueued = true;
      return;
    }
    _reloading = true;
    try {
      do {
        _reloadQueued = false;
        await loadCatalog();
        if (plans.isNotEmpty || plansLoading) await loadPlans();
      } while (_reloadQueued && !_shutDown);
    } finally {
      _reloading = false;
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

  /// Bumped by each [loadPlans] and device change: only the latest load
  /// writes [plans].
  int _plansGen = 0;
  String? profileId;
  Plan? get plan => profileId == null ? null : plans[profileId];

  // --------------------------------------------------------------- journey
  /// `first_time` or `returning` (see [Journey]).
  String? journeyId;
  Journey? get journey => journeyId == null ? null : plan?.journeys[journeyId];

  /// The mode Review starts with: a full install whenever the journey
  /// allows one. An update only works on a device that already runs a
  /// matching base, which the app can't see, so it's never the default.
  FlashMode get defaultMode => allowedModes.contains(FlashMode.full)
      ? FlashMode.full
      : allowedModes.first;

  /// Modes Review offers for the chosen journey, in the journey's order.
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

  /// The device and profile the current [buildRun] was started for.
  String? _builtDevice, _builtProfile;

  /// Bumped whenever the device or profile changes, so work started for
  /// the old selection (a space check before a build) doesn't carry on.
  int _selection = 0;

  /// The build finished and is for the device and profile chosen now.
  bool get buildCurrent =>
      buildRun.ok && _builtDevice == deviceId && _builtProfile == profileId;
  String? get job => buildCurrent ? buildRun.result?.str('job') : null;
  String? get image => buildCurrent ? buildRun.result?.str('image') : null;

  // ---------------------------------------------------------------- backup
  bool backedUp = false;

  // ---------------------------------------------------------------- unlock
  bool unlockConfirmed = false;
  final RunState unlockRun = RunState();
  EngineRun? _unlockHandle;
  String get unlockRoute => plan?.guides.unlockRoute ?? 'none';

  // --------------------------------------------------------------- connect
  List<DetectedDevice> detected = const [];
  EngineRun? _detectHandle;
  StreamSubscription<EngineEvent>? _detectSub;

  /// Why the device watch ended by itself (an engine error, or a local
  /// `detect_stopped` when it just ended). Cleared by [restartDetect].
  ErrorEvent? detectError;
  String? driverStatus;
  String? driverDetail;
  bool driverBusy = false;

  /// Why the last driver (or Linux device rule) install failed; the notice
  /// stays up with its hint. Cleared by the next check.
  ErrorEvent? driverError;

  /// Whether this computer needs a USB driver (Windows) or device rule
  /// (Linux) check; macOS never does.
  bool get checksDriver => host == 'windows' || host == 'linux';

  /// Bumped by each [checkDriver] and device change: only the latest check
  /// for the device chosen now writes the driver status.
  int _driverGen = 0;

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

  /// Set by [install]; cleared when the build, journey or mode changes.
  FlashConfirmation? _confirmed;

  // --------------------------------------------------------------- prepare
  /// Getting the build ready to install (`prepare`: the image checked, the
  /// partitions unpacked), started in the background as soon as the build
  /// finishes. Install then goes straight to asking for the device.
  final RunState prepareRun = RunState();
  EngineRun? _prepareHandle;

  /// The build [prepareRun] is for.
  String? _preparedJob;

  /// Install was confirmed while [prepareRun] was still going: the install
  /// starts as soon as it ends (whether or not it worked: the install then
  /// does any preparation still needed itself, and reports any problem).
  bool installPending = false;

  /// [prepareRun] is for the current build.
  bool get preparingCurrent => _preparedJob != null && _preparedJob == job;

  /// A build, unlock or install is running (or waiting to start): the
  /// selection and the step rail are locked until it ends.
  bool get busy =>
      buildRun.running ||
      flashRun.running ||
      unlockRun.running ||
      installPending;

  /// Set by [shutdown]: the app is closing, nothing new starts.
  bool _shutDown = false;

  /// A real install is past its checks and writing (or about to write) the
  /// device: stopping now needs a confirmation.
  bool get stopNeedsConfirmation =>
      flashRun.running &&
      !flashRunDry &&
      _writingStages.contains(flashRun.stage);

  /// Stages where the device is connected and being installed. Before them
  /// (checks, preparing, waiting for the device) nothing has been written,
  /// so stopping needs no confirmation.
  static const _writingStages = {'flash', 'backup', 'reboot'};

  // ----------------------------------------------------------------- clean
  /// What `clean --dry-run` listed for the workspace.
  List<CleanItem>? cleanItems;

  /// Kinds Done's "Free up space" removes (backups are never among them:
  /// they're deleted one by one after their own confirmation).
  final Map<String, bool> cleanChoice = {
    'scratch': true,
    'downloads': true,
    'image': false,
  };
  bool cleanBusy = false;

  /// Bytes the last clean freed; null until one worked.
  int? freedBytes;

  /// Why the last clean failed; null when it worked.
  ErrorEvent? cleanError;

  /// The job folder name a build for the current selection uses.
  String? get _selectedJobName =>
      deviceId == null || profileId == null ? null : '$deviceId-$profileId';

  /// What "Free up space…" may offer: scratch, downloads and images, never
  /// backups. With [keepCurrentImage] (freeing room to build or flash),
  /// the image of the current selection is kept: only other jobs' images.
  List<CleanItem> freeableItems({bool keepCurrentImage = true}) => [
    for (final i in cleanItems ?? const <CleanItem>[])
      if (i.kind == 'scratch' ||
          i.kind == 'downloads' ||
          (i.kind == 'image' &&
              !(keepCurrentImage && i.jobName == _selectedJobName)))
        i,
  ];

  /// The workspace's backup run folders (from the dry run).
  List<CleanItem> get backupItems => [
    for (final i in cleanItems ?? const <CleanItem>[])
      if (i.kind == 'backup') i,
  ];

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

  Future<void> selectDevice(String id) async {
    if (busy || deviceId == id) return;
    deviceId = id;
    plans.clear();
    _plansGen++;
    plansLoading = false;
    planError = null;
    profileId = null;
    exportOnly = false;
    detected = const [];
    driverStatus = driverDetail = null;
    driverError = null;
    driverBusy = false;
    _driverGen++;
    final done = _resetFrom(WizardStep.profile);
    notifyListeners();
    await done;
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
    if (buildCurrent) {
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
    final gen = ++_plansGen;
    plansLoading = true;
    planError = null;
    notifyListeners();
    // Collected apart and kept only if nothing changed meanwhile (another
    // device, or a newer load): plans never mix two devices.
    final loaded = <String, Plan>{};
    Object? error;
    try {
      for (final p in cat.profiles) {
        loaded[p.id] = await engine.resolve(device: d, profile: p.id);
        if (gen != _plansGen) return;
      }
    } catch (e) {
      error = e;
    }
    if (gen != _plansGen || deviceId != d) return;
    plans
      ..clear()
      ..addAll(loaded);
    planError = error;
    if (error == null) {
      profileId ??= cat.profiles
          .where((p) => p.available && (plans[p.id]?.available ?? false))
          .map((p) => p.id)
          .firstOrNull;
    }
    plansLoading = false;
    notifyListeners();
  }

  Future<void> selectProfile(String id) async {
    if (busy || !(plans[id]?.available ?? false) || profileId == id) return;
    profileId = id;
    final done = _resetFrom(WizardStep.build);
    notifyListeners();
    await done;
  }

  void selectJourney(String id) {
    if (busy || plan?.journeys[id] == null) return;
    if (journeyId != id) {
      journeyId = id;
      flashMode = defaultMode;
      unlockConfirmed = false;
      // The unlock step may now be needed, and the confirmed install no
      // longer matches: go through the steps after the journey again.
      _invalidateFlash(WizardStep.journey);
    }
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
      final picked = imports;
      final info = await engine.space(
        device: d,
        profile: p,
        workspace: w,
        imports: picked,
        deleteDownloads: deleteDownloads,
      );
      if (workspace.trim() == w &&
          deviceId == d &&
          profileId == p &&
          identical(imports, picked)) {
        space = info;
      }
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
      // Only the install the user confirmed, in its mode (dry or real).
      if (canRunAgain) {
        _startConfirmed();
      } else {
        goTo(WizardStep.review);
      }
    } else {
      await startBuild();
    }
  }

  /// Switches back to downloading. (Choosing files goes through
  /// [setImportFiles].)
  void useDownload() {
    if (imageSource == ImageSource.download) return;
    imageSource = ImageSource.download;
    notifyListeners();
    _recheckSpaceSoon();
  }

  /// The picked files changed what the build downloads: check again.
  void _recheckSpaceSoon() {
    if (workspace.trim().isEmpty || deviceId == null || profileId == null) {
      return;
    }
    space = null;
    checkSpace();
  }

  /// Uses [files] (one `.tar`, or its numbered parts) instead of
  /// downloading. An empty list keeps the current choice.
  void setImportFiles(List<String> files) {
    if (files.isEmpty) return;
    importFiles = List.unmodifiable([...files]..sort());
    imageSource = ImageSource.files;
    notifyListeners();
    _recheckSpaceSoon();
  }

  /// Checks free space first; never starts the build without room.
  Future<void> startBuild() async {
    if (busy || _shutDown || deviceId == null || profileId == null) return;
    if (workspace.trim().isEmpty || !imageSourceReady) return;
    final d = deviceId!, p = profileId!, sel = _selection;
    _spaceDebounce?.cancel();
    final info = await checkSpace();
    // The device or profile changed (or a build started) while checking.
    if (sel != _selection || deviceId != d || profileId != p || busy) return;
    if (_shutDown) return;
    if (info == null) return;
    if (!info.ok) {
      spaceProblem = SpaceProblem(
        SpaceProblemKind.build,
        need: info.need,
        free: info.free,
        reason: info.reason,
      );
      notifyListeners();
      return;
    }
    _invalidateFlash();
    // A new build replaces the image being prepared (and both need the
    // workspace).
    await _cancelPrepare();
    if (sel != _selection || busy || _shutDown) return;
    buildRun.reset();
    buildRun.running = true;
    _builtDevice = d;
    _builtProfile = p;
    notifyListeners();
    final run = engine.build(
      device: d,
      profile: p,
      workspace: workspace.trim(),
      imports: imports,
      deleteDownloads: deleteDownloads,
    );
    _buildHandle = run;
    _follow(
      run,
      buildRun,
      onDone: () {
        if (!identical(_buildHandle, run)) return;
        _buildHandle = null;
        if (buildCurrent) _startPrepare();
      },
    );
  }

  /// Starts [prepareRun] for the current build, unless this computer only
  /// builds (export only, or it can't install this device).
  void _startPrepare() {
    final j = job;
    if (j == null || _shutDown || exportOnly || onAndroid) return;
    if (!(device?.worksOn(host) ?? false)) return;
    if (prepareRun.running && _preparedJob == j) return;
    prepareRun.reset();
    prepareRun.running = true;
    _preparedJob = j;
    notifyListeners();
    final run = engine.prepare(job: j, mode: flashMode);
    _prepareHandle = run;
    _follow(
      run,
      prepareRun,
      onDone: () {
        if (!identical(_prepareHandle, run)) return;
        _prepareHandle = null;
        if (installPending) {
          installPending = false;
          _startConfirmed();
        }
      },
    );
  }

  /// Stops [prepareRun] (waiting for its engine to exit, so the workspace
  /// is free) and drops a pending install.
  Future<void> _cancelPrepare() async {
    final h = _prepareHandle;
    _prepareHandle = null;
    installPending = false;
    _preparedJob = null;
    prepareRun.reset();
    await h?.cancel();
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
    if (busy || _shutDown || deviceId == null) return;
    unlockRun.reset();
    unlockRun.running = true;
    notifyListeners();
    final run = engine.unlock(device: deviceId!);
    _unlockHandle = run;
    _follow(
      run,
      unlockRun,
      onDone: () {
        if (identical(_unlockHandle, run)) _unlockHandle = null;
      },
    );
  }

  void cancelUnlock() {
    _unlockHandle?.cancel();
  }

  void startDetect() {
    if (_detectHandle != null || _shutDown) return;
    detectError = null;
    final run = engine.detect();
    _detectHandle = run;
    // The watch ended by itself (the engine reported an error, exited or
    // crashed): what it saw is no longer known, so forget it and say so.
    // Nothing happens for a watch that was stopped or replaced meanwhile.
    void ended(ErrorEvent error) {
      if (!identical(_detectHandle, run)) return;
      final sub = _detectSub;
      _detectHandle = null;
      _detectSub = null;
      detected = const [];
      detectError = error;
      notifyListeners();
      run.cancel();
      sub?.cancel();
    }

    _detectSub = run.events.listen(
      (e) {
        if (!identical(_detectHandle, run)) return;
        if (e is DevicesEvent) {
          detected = e.devices;
          notifyListeners();
        } else if (e is ErrorEvent) {
          ended(e);
        }
      },
      onDone: () => ended(
        const ErrorEvent(code: 'detect_stopped', message: '', local: true),
      ),
    );
    if (checksDriver && deviceId != null) checkDriver();
  }

  /// "Try again" after the watch ended by itself.
  void restartDetect() {
    if (_detectHandle != null) return;
    startDetect();
    notifyListeners();
  }

  /// Stops the watch and forgets what it saw, so no later step relies on
  /// an old snapshot.
  Future<void> stopDetect() async {
    final h = _detectHandle, sub = _detectSub;
    _detectHandle = null;
    _detectSub = null;
    detected = const [];
    detectError = null;
    await h?.cancel();
    await sub?.cancel();
  }

  Future<void> checkDriver({bool install = false}) async {
    final d = deviceId;
    if (d == null) return;
    // The answer is for this device: dropped if another is chosen (or a
    // newer check starts) before it comes.
    final gen = ++_driverGen;
    bool current() => gen == _driverGen && deviceId == d;
    driverBusy = true;
    driverError = null;
    notifyListeners();
    String? status, detail;
    ErrorEvent? error;
    try {
      final r = await engine.drivers(device: d, install: install);
      status = r.str('status');
      detail = r.str('detail');
    } on EngineException catch (e) {
      // A failed check or install (declined UAC or password, no pkexec):
      // the notice stays, with the engine's hint.
      status = 'missing';
      detail = e.event.hint ?? e.event.message;
      error = e.event;
    } finally {
      if (current()) {
        driverStatus = status;
        driverDetail = detail;
        driverError = error;
        driverBusy = false;
        notifyListeners();
      }
    }
  }

  /// Dry run on Review. A change from what was confirmed means the
  /// confirmed install (and its result) no longer applies: a real install
  /// can't be reported, or repeated, as a dry run and the reverse.
  void setDryRun(bool v) {
    if (busy || dryRun == v) return;
    dryRun = v;
    final c = _confirmed;
    if (c != null && c.dry != v) _invalidateFlash();
    notifyListeners();
  }

  void setFlashMode(FlashMode m) {
    if (busy || !allowedModes.contains(m) || flashMode == m) return;
    flashMode = m;
    _invalidateFlash();
    notifyListeners();
  }

  /// Whether Review may start a real install: devices that must already be
  /// connected (not Unisoc, whose tool waits) need to have been detected.
  bool get canInstallForReal =>
      plan?.connectFirst == false || matchedDevice != null;

  /// Whether "Run again" may repeat the install confirmed on Review: the
  /// same build, mode and dry/real choice.
  bool get canRunAgain {
    final c = _confirmed;
    return c != null && c.job == job && !busy;
  }

  /// Repeats exactly what was confirmed on Review (never switches a dry run
  /// to a real install, or one mode or build for another). When that no
  /// longer applies, goes back to Review instead.
  void runAgain() {
    if (busy) return;
    if (!canRunAgain) {
      goTo(WizardStep.review);
      return;
    }
    _startOrWait();
  }

  /// Review's Install button (after the confirmation dialog): records what
  /// was confirmed, then starts it.
  void install() {
    final j = job;
    if (busy || j == null || (!dryRun && !canInstallForReal)) return;
    _confirmed = FlashConfirmation(j, flashMode, dryRun);
    _advanceTo(WizardStep.install);
    _startOrWait();
  }

  /// Starts the confirmed install, or, while the build is still being got
  /// ready, has it start when that ends (the install screen shows the
  /// preparation meanwhile). A dry run needs none of it.
  void _startOrWait() {
    final c = _confirmed;
    if (busy || _shutDown || c == null || c.job != job) return;
    if (!c.dry && prepareRun.running && preparingCurrent) {
      flashRun.reset();
      flashRunDry = false;
      installPending = true;
      notifyListeners();
      return;
    }
    _startConfirmed();
  }

  void _startConfirmed() {
    final c = _confirmed;
    if (busy || _shutDown || c == null || c.job != job) return;
    flashRun.reset();
    flashRun.running = true;
    flashRunDry = c.dry;
    notifyListeners();
    final run = engine.flash(job: c.job, dryRun: c.dry, mode: c.mode);
    _flashHandle = run;
    _follow(
      run,
      flashRun,
      onDone: () {
        if (identical(_flashHandle, run)) _flashHandle = null;
      },
    );
  }

  /// "Switch to full install" after the engine refused an update
  /// (`full_install_needed`): Review again, in full mode, with a new
  /// confirmation. Nothing is started here.
  void switchToFullInstall() {
    if (busy || !allowedModes.contains(FlashMode.full)) return;
    flashMode = FlashMode.full;
    _invalidateFlash();
    goTo(WizardStep.review);
    notifyListeners();
  }

  /// The build, journey or mode changed: the confirmed install and its
  /// result no longer apply, and Review must be passed again.
  void _invalidateFlash([WizardStep back = WizardStep.review]) {
    _confirmed = null;
    if (!flashRun.running) flashRun.reset();
    if (furthest.index > back.index) furthest = back;
  }

  /// Stop on the install screen: stops the install, or (while it waits for
  /// the preparation) keeps it from starting. The preparation itself goes
  /// on in the background.
  Future<void> cancelFlash() async {
    if (installPending) {
      installPending = false;
      notifyListeners();
      return;
    }
    await _flashHandle?.cancel();
  }

  /// The app is closing: stops the install, build and unlock (waiting for
  /// each engine to be told) and the device watch, and starts nothing new.
  /// Safe to call more than once.
  Future<void> shutdown() async {
    _shutDown = true;
    _spaceDebounce?.cancel();
    final handles = [_flashHandle, _buildHandle, _unlockHandle];
    await Future.wait([
      for (final h in handles)
        if (h != null) h.cancel(),
      stopDetect(),
      _cancelPrepare(),
    ]);
  }

  /// Lists what can be removed (`clean --dry-run`). Clears the last
  /// clean's result unless [keepResult].
  Future<void> loadCleanItems({bool keepResult = false}) async {
    if (workspace.trim().isEmpty) return;
    if (!keepResult) {
      freedBytes = null;
      cleanError = null;
    }
    try {
      final r = await engine.clean(workspace: workspace.trim(), dryRun: true);
      cleanItems = CleanItem.listFrom(r);
    } on EngineException catch (e) {
      cleanItems = const [];
      cleanError ??= e.event;
    }
    notifyListeners();
  }

  void setCleanChoice(String kind, bool v) {
    if (!cleanChoice.containsKey(kind)) return;
    cleanChoice[kind] = v;
    notifyListeners();
  }

  /// Removes exactly [paths] (items from the dry run, `clean --path`), then
  /// lists again. True when the engine removed them; otherwise
  /// [cleanError] says why and [freedBytes] is null.
  Future<bool> cleanPaths(List<String> paths) async {
    if (cleanBusy || workspace.trim().isEmpty || paths.isEmpty) return false;
    cleanBusy = true;
    cleanError = null;
    freedBytes = null;
    notifyListeners();
    // Cleaning needs the workspace: a preparation still running stops (the
    // install does any preparation still needed itself).
    if (prepareRun.running) await _cancelPrepare();
    var ok = false;
    try {
      final r = await engine.clean(workspace: workspace.trim(), paths: paths);
      final freed = r.data['freed'];
      freedBytes = freed is num ? freed.toInt() : 0;
      ok = true;
    } on EngineException catch (e) {
      cleanError = e.event;
    } finally {
      cleanBusy = false;
      notifyListeners();
    }
    await loadCleanItems(keepResult: true);
    return ok;
  }

  /// Done's "Free up": the items of the chosen kinds (never backups).
  Future<bool> runClean() => cleanPaths([
    for (final i in freeableItems(keepCurrentImage: false))
      if (cleanChoice[i.kind] ?? false) i.path,
  ]);

  /// Deletes the backup run folders the user picked and confirmed; any
  /// other path is ignored.
  Future<bool> deleteBackups(Iterable<String> paths) {
    final backups = {for (final b in backupItems) b.path};
    return cleanPaths([
      for (final p in paths)
        if (backups.contains(p)) p,
    ]);
  }

  /// "Copy details": the engine's stderr log and [run]'s log.
  String diagnostics([RunState? run]) {
    final b = StringBuffer()
      ..writeln('Thimsy Installer diagnostics')
      ..writeln('device: ${deviceId ?? '-'}  profile: ${profileId ?? '-'}')
      ..writeln('host: $host  language: ${engine.language}');
    if (run != null) {
      b.writeln('\n== run log');
      for (final line in run.log) {
        b.writeln(line);
      }
    }
    b.writeln('\n== engine log (stderr)');
    for (final line in engine.stderrLog) {
      b.writeln(line);
    }
    return b.toString();
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

  /// Folds [run]'s events into [state] until [state] is reset (the run was
  /// replaced or abandoned); later events are dropped.
  void _follow(EngineRun run, RunState state, {VoidCallback? onDone}) {
    final generation = state.generation;
    bool current() => state.generation == generation;
    run.events.listen(
      (e) {
        if (!current()) return;
        state.apply(e);
        // The background preparation never interrupts with a dialog: the
        // install repeats anything that failed and reports it there.
        if (identical(state, prepareRun)) {
          notifyListeners();
          return;
        }
        if (e is ErrorEvent && e.code == 'unsupported_host') hostError = e;
        if (e is ErrorEvent && e.code == 'not_enough_space') {
          spaceProblem = SpaceProblem(
            identical(state, flashRun)
                ? SpaceProblemKind.flash
                : SpaceProblemKind.build,
            message: e.hint ?? e.message,
            details: e.message,
          );
        }
        notifyListeners();
      },
      onDone: () {
        if (current()) state.running = false;
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
        return buildCurrent;
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
        // A dry run wrote nothing: there's no installed Thimsy to go on to.
        return flashRun.ok && !flashRunDry;
      case WizardStep.firstBoot:
        return true;
      case WizardStep.done:
        return false;
    }
  }

  /// Generic Next (the device step uses [deviceNext]).
  void next() {
    if (busy || !canProceed(step)) return;
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

  /// Steps can't be changed while a build or install runs.
  bool canGoTo(WizardStep s) =>
      !busy && s.index <= furthest.index && !isSkipped(s);

  void goTo(WizardStep s) {
    if (!canGoTo(s)) return;
    _enter(s);
  }

  void _advanceTo(WizardStep s) {
    if (s.index > furthest.index) furthest = s;
    _enter(s);
  }

  /// Steps that watch for the device: Review keeps watching, so what it
  /// shows (and allows) is what's connected now.
  static const _detectSteps = {WizardStep.connect, WizardStep.review};

  void _enter(WizardStep s) {
    final leaving = step;
    step = s;
    if (_detectSteps.contains(leaving) && !_detectSteps.contains(s)) {
      stopDetect();
    }
    if (_detectSteps.contains(s)) startDetect();
    if (s == WizardStep.profile && plans.isEmpty) loadPlans();
    if (s == WizardStep.build && space == null && workspace.isNotEmpty) {
      checkSpace();
    }
    if (s == WizardStep.done) loadCleanItems();
    notifyListeners();
  }

  /// Forget progress from [s] on (a different device or profile was
  /// picked). Stops and waits for anything still running for the old
  /// selection; their remaining events are ignored.
  Future<void> _resetFrom(WizardStep s) async {
    _selection++;
    if (furthest.index >= s.index) furthest = WizardStep.values[s.index - 1];
    if (s.index <= WizardStep.profile.index) journeyId = null;
    if (s.index <= WizardStep.build.index) {
      buildRun.reset();
      _builtDevice = _builtProfile = null;
      space = null;
    }
    backedUp = false;
    unlockConfirmed = false;
    unlockRun.reset();
    flashRun.reset();
    _confirmed = null;
    firstBootChecks.clear();
    final handles = [_buildHandle, _flashHandle, _unlockHandle];
    _buildHandle = _flashHandle = _unlockHandle = null;
    await Future.wait([
      for (final h in handles)
        if (h != null) h.cancel(),
      stopDetect(),
      _cancelPrepare(),
    ]);
  }

  @override
  void dispose() {
    shutdown();
    super.dispose();
  }
}
