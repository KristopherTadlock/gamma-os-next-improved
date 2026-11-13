// The stand-in engine is a POSIX shell script.
@TestOn('!windows')
library;

import 'dart:async';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:thimsy_installer/engine/engine.dart';
import 'package:thimsy_installer/engine/process_engine.dart';

/// A stand-in engine: a shell script that answers like `thimsy-installer`.
const _script = r'''#!/bin/sh
echo "args: $*" >&2
case "$1" in
  catalog)
    echo '{"type":"catalog","version":"9","host":"linux","devices":[],"profiles":[]}'
    ;;
  build)
    echo '{"type":"stage","stage":"fetch","title":"Downloading"}'
    echo 'a stray tool line'
    printf '{"type":"progress","stage":"fetch","label":"f","done":1,'
    printf '"total":2}\n'
    echo "{\"type\":\"log\",\"level\":\"info\",\"message\":\"$*\"}"
    echo '{"type":"result","ok":true,"job":"/w/jobs/j","image":"/w/i.pac"}'
    ;;
  flash)
    if [ "$3" = hang ]; then
      # A tool left running in the background holds stdout open; the
      # engine itself then hangs until killed.
      echo '{"type":"stage","stage":"check","title":"Checking"}'
      sleep 30 &
      echo $! > "$(dirname "$0")/bg.pid"
      exec sleep 30
    fi
    echo '{"type":"error","code":"unsupported_host","message":"Needs Windows","hint":"Use a Windows PC"}'
    echo '{"type":"result","ok":false}'
    ;;
  resolve)
    echo '{"type":"error","code":"profile_unavailable","message":"No such profile"}'
    exit 2
    ;;
  detect)
    exec sleep 30
    ;;
  drivers)
    case "$*" in
      *--install*)
        # A declined install: an error, then a failed result.
        echo '{"type":"error","code":"tool_failed","message":"pkexec: request dismissed","hint":"Approve the password prompt to allow access."}'
        echo '{"type":"result","ok":false}'
        exit 1 ;;
      *) echo '{"type":"result","ok":true,"status":"missing","detail":"no udev rule"}' ;;
    esac
    ;;
  clean)
    case "$*" in
      *--dry-run*)
        echo '{"type":"result","ok":true,"items":[{"kind":"scratch","path":"/w/jobs/j/work","bytes":5}],"freed":0}' ;;
      *--path\ /w/locked*)
        echo '{"type":"error","code":"internal","message":"Permission denied (os error 13)"}'
        echo '{"type":"result","ok":false}'
        exit 1 ;;
      *--path\ /w/silent*)
        # A failed result with no error line.
        echo '{"type":"result","ok":false}'
        exit 1 ;;
      *) echo '{"type":"result","ok":true,"items":[],"freed":5}' ;;
    esac
    ;;
  space)
    echo '{"type":"space","need":0,"free":0,"ok":false,"reason":"fat32"}'
    ;;
  unlock)
    # Bytes that aren't UTF-8, then an exit without a result.
    [ "$3" = garbage ] && printf '\377\376\n' && exit 0
    exit 3
    ;;
  *)
    exit 3
    ;;
esac
''';

void main() {
  late Directory dir;
  late ProcessEngine engine;

  setUp(() async {
    dir = await Directory.systemTemp.createTemp('thimsy-engine-test');
    final f = File('${dir.path}/thimsy-installer');
    await f.writeAsString(_script);
    await Process.run('chmod', ['+x', f.path]);
    engine = ProcessEngine(f.path);
  });

  tearDown(() => dir.delete(recursive: true));

  test('catalog parses the one-line answer', () async {
    final cat = await engine.catalog();
    expect(cat.version, '9');
    expect(engine.stderrLog.single, 'args: catalog --lang en --json');
  });

  test('build streams events and passes --json and arguments', () async {
    final events = await engine
        .build(
          device: 'd',
          profile: 'medium',
          workspace: '/w',
          imports: const ['/dl/p.tar.001', '/dl/p.tar.002'],
        )
        .events
        .toList();
    expect(events.first, isA<StageEvent>());
    expect(events[1], isA<LogEvent>());
    expect((events[2] as ProgressEvent).fraction, 0.5);
    expect(
      (events[3] as LogEvent).message,
      'build --device d --profile medium --workspace /w '
      '--import /dl/p.tar.001 --import /dl/p.tar.002 --lang en --json',
    );
    expect((events.last as ResultEvent).str('image'), '/w/i.pac');
  });

  test(
    'flash passes mode and dry-run, and surfaces unsupported_host',
    () async {
      final events = await engine
          .flash(job: '/w/jobs/j', dryRun: true, mode: FlashMode.update)
          .events
          .toList();
      expect(
        engine.stderrLog.last,
        'args: flash --job /w/jobs/j --mode update --dry-run --lang en --json',
      );
      final err = events.whereType<ErrorEvent>().single;
      expect(err.code, 'unsupported_host');
      expect((events.last as ResultEvent).ok, isFalse);
    },
  );

  test('the chosen language is passed as --lang on every command', () async {
    engine.language = 'zh-tw';
    await engine.catalog();
    expect(engine.stderrLog.last, 'args: catalog --lang zh-tw --json');
    await engine
        .build(
          device: 'd',
          profile: 'p',
          workspace: '/w',
          deleteDownloads: true,
        )
        .events
        .toList();
    expect(
      engine.stderrLog.last,
      'args: build --device d --profile p --workspace /w --delete-downloads '
      '--lang zh-tw --json',
    );
  });

  test('a one-shot error becomes an EngineException', () async {
    expect(
      () => engine.resolve(device: 'd', profile: 'nope'),
      throwsA(
        isA<EngineException>().having(
          (e) => e.code,
          'code',
          'profile_unavailable',
        ),
      ),
    );
  });

  test('a crash without a result adds an error and a failed result', () async {
    final events = await engine.unlock(device: 'd').events.toList();
    expect(events.whereType<ErrorEvent>().single.code, 'engine_crashed');
    expect((events.last as ResultEvent).ok, isFalse);
  });

  test('a cancel before the process has started still stops it', () async {
    final run = engine.detect();
    // Synchronously: Process.start hasn't returned yet.
    unawaited(run.cancel());
    final events = await run.events.toList().timeout(
      const Duration(seconds: 10),
      onTimeout: () => fail('the engine process kept running'),
    );
    expect(events.whereType<ErrorEvent>().single.code, 'cancelled');
    expect((events.last as ResultEvent).ok, isFalse);
  });

  // Custom settings aren't built into the image yet: `build` has no way to
  // take them (see the build test's exact arguments), previews still work.
  test('settings: resolve still passes overrides as --set', () async {
    await expectLater(
      engine.resolve(
        device: 'd',
        profile: 'nope',
        overrides: const {'nano.theme': 'grid'},
      ),
      throwsA(isA<EngineException>()),
    );
    expect(
      engine.stderrLog.last,
      'args: resolve --device d --profile nope --set nano.theme=grid '
      '--lang en --json',
    );
  });

  test('a missing engine binary reports engine_missing', () async {
    final missing = ProcessEngine('${dir.path}/nope');
    final events = await missing.detect().events.toList();
    expect(events.whereType<ErrorEvent>().single.code, 'engine_missing');
  });

  test('a cancelled run ends when the engine exits, even with stdout held '
      'open by a tool it started', () async {
    final bgPid = File('${dir.path}/bg.pid');
    addTearDown(() async {
      if (await bgPid.exists()) {
        Process.killPid(int.parse((await bgPid.readAsString()).trim()));
      }
    });
    final run = engine.flash(job: 'hang');
    final events = <EngineEvent>[];
    final firstEvent = Completer<void>();
    final done = Completer<void>();
    run.events.listen((e) {
      events.add(e);
      if (!firstEvent.isCompleted) firstEvent.complete();
    }, onDone: done.complete);
    await firstEvent.future.timeout(const Duration(seconds: 10));
    // The background tool has been started by now (its pid is written
    // before the engine execs).
    for (var i = 0; i < 100 && !await bgPid.exists(); i++) {
      await Future<void>.delayed(const Duration(milliseconds: 20));
    }
    final watch = Stopwatch()..start();
    await run.cancel();
    await done.future.timeout(
      const Duration(seconds: 10),
      onTimeout: () => fail('the run only ended when stdout closed'),
    );
    expect(watch.elapsed, lessThan(const Duration(seconds: 10)));
    expect(events.first, isA<StageEvent>());
    expect(events.whereType<ErrorEvent>().single.code, 'cancelled');
    expect((events.last as ResultEvent).ok, isFalse);
  });

  test(
    'bytes that are not UTF-8 still end with an error and a result',
    () async {
      final events = await engine
          .unlock(device: 'garbage')
          .events
          .toList()
          .timeout(const Duration(seconds: 10));
      // The garbage line is kept (as replacement characters), not fatal.
      expect(events.first, isA<LogEvent>());
      expect((events.first as LogEvent).message, '��');
      expect(events.whereType<ErrorEvent>().single.code, 'engine_crashed');
      expect((events.last as ResultEvent).ok, isFalse);
    },
  );

  group('failed results of one-shot commands are never swallowed', () {
    test('a failed driver install throws with the engine\'s error', () async {
      final status = await engine.drivers(device: 'd');
      expect(status.str('status'), 'missing');
      await expectLater(
        engine.drivers(device: 'd', install: true),
        throwsA(
          isA<EngineException>()
              .having((e) => e.code, 'code', 'tool_failed')
              .having(
                (e) => e.event.hint,
                'hint',
                'Approve the password prompt to allow access.',
              ),
        ),
      );
      expect(
        engine.stderrLog.last,
        'args: drivers --device d --install --lang en --json',
      );
    });

    test('a failed clean throws, with or without an error line', () async {
      final ok = await engine.clean(workspace: '/w', paths: const ['/w/a']);
      expect(ok.data['freed'], 5);
      await expectLater(
        engine.clean(workspace: '/w', paths: const ['/w/locked']),
        throwsA(
          isA<EngineException>().having((e) => e.code, 'code', 'internal'),
        ),
      );
      await expectLater(
        engine.clean(workspace: '/w', paths: const ['/w/silent']),
        throwsA(isA<EngineException>()),
      );
    });
  });

  test('clean --path is repeated, one per item, and nothing else', () async {
    await engine.clean(
      workspace: '/w',
      paths: const ['/w/jobs/j/work', '/w/cache/a b.7z'],
    );
    expect(
      engine.stderrLog.last,
      'args: clean --workspace /w --path /w/jobs/j/work --path /w/cache/a b.7z '
      '--lang en --json',
    );
  });

  test('space passes the picked files as --import and reads reason', () async {
    final info = await engine.space(
      device: 'd',
      profile: 'p',
      workspace: '/w',
      imports: const ['/dl/p.tar.001', '/dl/p.tar.002'],
    );
    expect(
      engine.stderrLog.last,
      'args: space --device d --profile p --workspace /w '
      '--import /dl/p.tar.001 --import /dl/p.tar.002 --lang en --json',
    );
    expect(info.ok, isFalse);
    expect(info.reason, 'fat32');
  });
}
