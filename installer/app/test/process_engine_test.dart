// The stand-in engine is a POSIX shell script.
@TestOn('!windows')
library;

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
    echo '{"type":"error","code":"unsupported_host","message":"Needs Windows","hint":"Use a Windows PC"}'
    echo '{"type":"result","ok":false}'
    ;;
  resolve)
    echo '{"type":"error","code":"profile_unavailable","message":"No such profile"}'
    exit 2
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
          importDir: '/dl',
        )
        .events
        .toList();
    expect(events.first, isA<StageEvent>());
    expect(events[1], isA<LogEvent>());
    expect((events[2] as ProgressEvent).fraction, 0.5);
    expect(
      (events[3] as LogEvent).message,
      'build --device d --profile medium --workspace /w --import /dl '
      '--lang en --json',
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

  test('a missing engine binary reports tool_missing', () async {
    final missing = ProcessEngine('${dir.path}/nope');
    final events = await missing.detect().events.toList();
    expect(events.whereType<ErrorEvent>().single.code, 'engine_missing');
  });
}
