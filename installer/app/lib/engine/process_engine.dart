import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:path/path.dart' as p;

import 'engine.dart';

/// Runs the bundled `thimsy-installer` engine and reads its JSON Lines.
///
/// The binary is found at `THIMSY_ENGINE` if set, else next to the app:
/// `<dir of the app executable>/engine/thimsy-installer[.exe]`.
class ProcessEngine extends Engine {
  final String executable;

  /// Lines the engine printed on stderr, kept for the diagnostic bundle.
  final List<String> stderrLog = [];

  ProcessEngine([String? executable])
    : executable = executable ?? locateEngine();

  static String locateEngine() {
    final env = Platform.environment['THIMSY_ENGINE'];
    if (env != null && env.isNotEmpty) return env;
    final dir = p.dirname(Platform.resolvedExecutable);
    final name = Platform.isWindows
        ? 'thimsy-installer.exe'
        : 'thimsy-installer';
    return p.join(dir, 'engine', name);
  }

  /// Starts a command and returns its events. When the process ends without
  /// a `result` line, an error and a failed result are added.
  _ProcessRun _start(List<String> args) => _ProcessRun.start(executable, [
    ...args,
    '--lang',
    language,
    '--json',
  ], onStderr: stderrLog.add);

  /// Runs a one-shot command; returns the first event matching [want].
  Future<T> _one<T extends EngineEvent>(List<String> args) async {
    final run = _start(args);
    T? found;
    ErrorEvent? error;
    await for (final e in run.events) {
      if (e is T && found == null) found = e;
      if (e is ErrorEvent) error ??= e;
    }
    if (found != null) return found;
    throw EngineException(
      error ??
          ErrorEvent(
            code: 'no_answer',
            message: 'no answer for ${args.first}',
            local: true,
          ),
    );
  }

  static List<String> _sets(Map<String, String> overrides) => [
    for (final e in overrides.entries) ...['--set', '${e.key}=${e.value}'],
  ];

  @override
  Future<Catalog> catalog() async =>
      (await _one<CatalogEvent>(['catalog'])).catalog;

  @override
  Future<Plan> resolve({
    required String device,
    required String profile,
    Map<String, String> overrides = const {},
  }) async => (await _one<PlanEvent>([
    'resolve',
    '--device',
    device,
    '--profile',
    profile,
    ..._sets(overrides),
  ])).plan;

  @override
  Future<SpaceInfo> space({
    required String device,
    required String profile,
    required String workspace,
    bool deleteDownloads = false,
  }) async => (await _one<SpaceEvent>([
    'space',
    '--device',
    device,
    '--profile',
    profile,
    '--workspace',
    workspace,
    if (deleteDownloads) '--delete-downloads',
  ])).space;

  @override
  Future<ResultEvent> clean({
    required String workspace,
    bool dryRun = false,
    bool downloads = false,
    bool image = false,
    bool scratch = false,
    bool backup = false,
  }) => _one<ResultEvent>([
    'clean',
    '--workspace',
    workspace,
    if (dryRun) '--dry-run',
    if (downloads) '--downloads',
    if (image) '--image',
    if (scratch) '--scratch',
    if (backup) '--backup',
  ]);

  @override
  EngineRun build({
    required String device,
    required String profile,
    required String workspace,
    List<String> imports = const [],
    bool deleteDownloads = false,
    Map<String, String> overrides = const {},
  }) => _start([
    'build',
    '--device',
    device,
    '--profile',
    profile,
    '--workspace',
    workspace,
    for (final path in imports)
      if (path.isNotEmpty) ...['--import', path],
    if (deleteDownloads) '--delete-downloads',
    ..._sets(overrides),
  ]);

  @override
  EngineRun detect() => _start(['detect', '--watch']);

  @override
  EngineRun flash({
    required String job,
    bool dryRun = false,
    FlashMode mode = FlashMode.full,
  }) => _start([
    'flash',
    '--job',
    job,
    '--mode',
    mode.name,
    if (dryRun) '--dry-run',
  ]);

  @override
  EngineRun unlock({required String device}) =>
      _start(['unlock', '--device', device]);

  @override
  Future<ResultEvent> drivers({required String device, bool install = false}) =>
      _one<ResultEvent>([
        'drivers',
        '--device',
        device,
        if (install) '--install',
      ]);
}

class _ProcessRun implements EngineRun {
  final StreamController<EngineEvent> _out = StreamController();
  Process? _process;
  bool _cancelled = false;

  _ProcessRun._();

  factory _ProcessRun.start(
    String exe,
    List<String> args, {
    required void Function(String) onStderr,
  }) {
    final run = _ProcessRun._();
    run._run(exe, args, onStderr);
    return run;
  }

  Future<void> _run(
    String exe,
    List<String> args,
    void Function(String) onStderr,
  ) async {
    var sawResult = false;
    ErrorEvent? lastError;
    try {
      final proc = await Process.start(exe, args);
      _process = proc;
      final err = proc.stderr
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen(onStderr);
      await for (final e
          in proc.stdout
              .transform(utf8.decoder)
              .transform(const JsonLinesDecoder())) {
        if (e is ResultEvent) sawResult = true;
        if (e is ErrorEvent) lastError = e;
        _out.add(e);
      }
      final code = await proc.exitCode;
      await err.cancel();
      if (!sawResult) {
        if (_cancelled) {
          _out.add(
            const ErrorEvent(code: 'cancelled', message: '', local: true),
          );
        } else if (lastError == null) {
          _out.add(
            ErrorEvent(
              code: 'engine_crashed',
              message: 'exit code $code',
              local: true,
              args: {'exit': '$code'},
            ),
          );
        }
        _out.add(ResultEvent(false, {'type': 'result', 'ok': false}));
      }
    } on ProcessException catch (e) {
      _out.add(
        ErrorEvent(
          code: 'engine_missing',
          message: e.message,
          local: true,
          args: {'path': exe},
        ),
      );
      _out.add(ResultEvent(false, {'type': 'result', 'ok': false}));
    } finally {
      await _out.close();
    }
  }

  @override
  Stream<EngineEvent> get events => _out.stream;

  @override
  Future<void> cancel() async {
    _cancelled = true;
    _process?.kill();
  }
}
