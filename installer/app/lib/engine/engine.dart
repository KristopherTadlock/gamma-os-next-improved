import 'dart:async';

import 'events.dart';

export 'events.dart';

/// Thrown by one-shot engine calls when the engine reports an error.
class EngineException implements Exception {
  final ErrorEvent event;
  const EngineException(this.event);

  String get code => event.code;

  @override
  String toString() => event.hint == null
      ? '${event.code}: ${event.message}'
      : '${event.code}: ${event.message}\n${event.hint}';
}

/// A running streaming command (`build`, `flash`, `unlock`, `detect --watch`).
abstract class EngineRun {
  /// Events in order. Always ends with a [ResultEvent] (except `detect
  /// --watch`, which runs until [cancel]).
  Stream<EngineEvent> get events;

  /// Stops the command (kills the engine process).
  Future<void> cancel();
}

/// How `flash` writes the device.
enum FlashMode {
  /// Fresh install: every partition in the plan, erases user data.
  full,

  /// Update: only partitions that changed, keeps user data.
  update,
}

/// What the app needs from the installer engine. [ProcessEngine] runs the
/// bundled `thimsy-installer` binary; [FakeEngine] is for tests and demos.
abstract class Engine {
  /// Thimsy language code passed as `--lang` on every command, so the
  /// engine's stage titles, errors and guides come back translated.
  String language = 'en';

  Future<Catalog> catalog();

  Future<Plan> resolve({
    required String device,
    required String profile,
    Map<String, String> overrides = const {},
  });

  /// `space`: free space check for a build in [workspace].
  Future<SpaceInfo> space({
    required String device,
    required String profile,
    required String workspace,
    bool deleteDownloads = false,
  });

  /// Builds the image with the profile's settings as they are: custom
  /// settings (`--set`) aren't built into the image yet, and the engine
  /// refuses them, so there is no way to pass any here. [resolve] still
  /// takes overrides to preview them.
  EngineRun build({
    required String device,
    required String profile,
    required String workspace,
    List<String> imports = const [],
    bool deleteDownloads = false,
  });

  /// `detect --watch`: emits [DevicesEvent]s until cancelled.
  EngineRun detect();

  EngineRun flash({
    required String job,
    bool dryRun = false,
    FlashMode mode = FlashMode.full,
  });

  EngineRun unlock({required String device});

  /// `clean`: with [dryRun], lists what can be removed (`items`); otherwise
  /// removes the chosen kinds and reports `freed` bytes.
  Future<ResultEvent> clean({
    required String workspace,
    bool dryRun = false,
    bool downloads = false,
    bool image = false,
    bool scratch = false,
    bool backup = false,
  });

  /// Windows driver check (or install). Returns the `result` event.
  Future<ResultEvent> drivers({required String device, bool install = false});
}
