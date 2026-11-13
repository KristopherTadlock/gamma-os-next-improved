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

  /// Lines the engine printed on stderr, kept for "Copy details".
  @override
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

  /// Runs a one-shot command; returns the first event of type [T]. A
  /// failed `result` (`ok` not true) is never returned: it throws, with the
  /// engine's error when it sent one.
  Future<T> _one<T extends EngineEvent>(List<String> args) async {
    final run = _start(args);
    T? found;
    ErrorEvent? error;
    await for (final e in run.events) {
      if (e is T && found == null) found = e;
      if (e is ErrorEvent) error ??= e;
    }
    if (found != null && !(found is ResultEvent && !found.ok)) return found;
    if (found is ResultEvent) {
      throw EngineException(
        error ??
            ErrorEvent(
              code: 'internal',
              message: '${args.first} failed without an error',
            ),
      );
    }
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
    List<String> imports = const [],
    bool deleteDownloads = false,
  }) async => (await _one<SpaceEvent>([
    'space',
    '--device',
    device,
    '--profile',
    profile,
    '--workspace',
    workspace,
    for (final path in imports)
      if (path.isNotEmpty) ...['--import', path],
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
    List<String> paths = const [],
  }) => _one<ResultEvent>([
    'clean',
    '--workspace',
    workspace,
    if (dryRun) '--dry-run',
    if (downloads) '--downloads',
    if (image) '--image',
    if (scratch) '--scratch',
    if (backup) '--backup',
    for (final path in paths)
      if (path.isNotEmpty) ...['--path', path],
  ]);

  @override
  EngineRun build({
    required String device,
    required String profile,
    required String workspace,
    List<String> imports = const [],
    bool deleteDownloads = false,
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
  ]);

  @override
  EngineRun detect() => _start(['detect', '--watch']);

  @override
  EngineRun prepare({required String job, FlashMode mode = FlashMode.full}) =>
      _start(['prepare', '--job', job, '--mode', mode.name]);

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

  /// Completed by [cancel].
  final Completer<void> _cancelRequested = Completer<void>();

  /// Completed when the run has ended and its events are closed.
  final Completer<void> _ended = Completer<void>();

  /// How long a cancelled run's output is still read after the engine has
  /// exited (its last lines), before the stream is closed regardless: a
  /// tool the engine started may still hold stdout open.
  static const _drainAfterExit = Duration(milliseconds: 500);

  /// How long [cancel] waits for the engine to exit before killing it
  /// outright.
  static const _exitWait = Duration(seconds: 5);

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
    // Ends the run with an error and a failed result, unless it already
    // has its result.
    void fail(ErrorEvent e) {
      if (sawResult || _out.isClosed) return;
      _out.add(e);
      _out.add(ResultEvent(false, {'type': 'result', 'ok': false}));
      sawResult = true;
    }

    StreamSubscription<EngineEvent>? out;
    StreamSubscription<String>? err;
    try {
      final proc = await Process.start(exe, args);
      _process = proc;
      // Cancelled while the process was starting: stop it now.
      if (_cancelled) proc.kill();
      // Malformed UTF-8 (a tool's raw bytes) becomes U+FFFD, never an
      // error that would end the run without a result.
      err = proc.stderr
          .transform(const Utf8Decoder(allowMalformed: true))
          .transform(const LineSplitter())
          .listen(onStderr, onError: (Object _) {});
      final stdoutDone = Completer<void>();
      out = proc.stdout
          .transform(const Utf8Decoder(allowMalformed: true))
          .transform(const JsonLinesDecoder())
          .listen(
            (e) {
              if (e is ResultEvent) sawResult = true;
              if (e is ErrorEvent) lastError = e;
              _out.add(e);
            },
            onError: (Object e, StackTrace st) {
              if (!stdoutDone.isCompleted) stdoutDone.completeError(e, st);
            },
            onDone: () {
              if (!stdoutDone.isCompleted) stdoutDone.complete();
            },
            cancelOnError: true,
          );
      // Normally the run ends when the engine closes stdout. Once
      // cancelled, it ends when the engine exits: a tool it started may
      // keep stdout open long after.
      final exitedAfterCancel = _cancelRequested.future.then(
        (_) => proc.exitCode,
      );
      await Future.any([stdoutDone.future, exitedAfterCancel]);
      if (!stdoutDone.isCompleted) {
        await stdoutDone.future.timeout(_drainAfterExit, onTimeout: () {});
      }
      await out.cancel();
      out = null;
      final code = await proc.exitCode;
      await err.cancel();
      err = null;
      if (!sawResult) {
        if (_cancelled) {
          fail(const ErrorEvent(code: 'cancelled', message: '', local: true));
        } else if (lastError == null) {
          fail(
            ErrorEvent(
              code: 'engine_crashed',
              message: 'exit code $code',
              local: true,
              args: {'exit': '$code'},
            ),
          );
        } else {
          _out.add(ResultEvent(false, {'type': 'result', 'ok': false}));
        }
      }
    } on ProcessException catch (e) {
      fail(
        ErrorEvent(
          code: 'engine_missing',
          message: e.message,
          local: true,
          args: {'path': exe},
        ),
      );
    } catch (e) {
      // Anything else unexpected (unreadable output, a parse failure):
      // still an error and a failed result, never a silent end.
      fail(
        ErrorEvent(
          code: 'engine_crashed',
          message: '$e',
          local: true,
          args: {'exit': '?'},
        ),
      );
      _process?.kill();
    } finally {
      await out?.cancel();
      await err?.cancel();
      await _out.close();
      if (!_ended.isCompleted) _ended.complete();
    }
  }

  @override
  Stream<EngineEvent> get events => _out.stream;

  /// Kills the engine and waits (a few seconds at most) for the run to end;
  /// an engine that hasn't exited by then is killed outright.
  @override
  Future<void> cancel() async {
    _cancelled = true;
    if (!_cancelRequested.isCompleted) _cancelRequested.complete();
    _process?.kill();
    await _ended.future.timeout(
      _exitWait,
      onTimeout: () => _process?.kill(ProcessSignal.sigkill),
    );
  }
}
