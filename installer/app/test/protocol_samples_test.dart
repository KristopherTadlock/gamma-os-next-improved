// Contract test: real engine output (installer/docs/protocol-samples/*.jsonl,
// written by the engine) goes through the app's own decoder and models, and
// the fields the app relies on are there and parsed. When the engine
// changes a field, this fails here instead of in a user's install.
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:path/path.dart' as p;
import 'package:thimsy_installer/engine/engine.dart';
import 'package:thimsy_installer/engine/process_engine.dart';
import 'package:thimsy_installer/l10n/app_localizations.dart';
import 'package:thimsy_installer/widgets/common.dart';

/// Relative to installer/app (where `flutter test` runs).
const samplesDir = '../docs/protocol-samples';

/// The samples the engine side provides; each must exist.
const expected = [
  'catalog.jsonl',
  'resolve-rg-cube-heavy.jsonl',
  'space.jsonl',
  'flash-dry-run.jsonl',
  'prepare.jsonl',
  'flash-full.jsonl',
  'detect.jsonl',
  'error-wrong-device.jsonl',
  'error-device-not-found.jsonl',
  'drivers-failed.jsonl',
];

class Sample {
  final String name;
  final List<Map<String, dynamic>> raw;
  final List<EngineEvent> events;
  Sample(this.name, this.raw, this.events);

  Iterable<T> of<T extends EngineEvent>() => events.whereType<T>();

  /// The raw JSON of the first line of [type].
  Map<String, dynamic> rawOf(String type) => raw.firstWhere(
    (j) => j['type'] == type,
    orElse: () => fail('$name has no "$type" line'),
  );
}

Sample load(File f) {
  final name = p.basename(f.path);
  final raw = <Map<String, dynamic>>[];
  final events = <EngineEvent>[];
  final lines = f
      .readAsLinesSync()
      .map((l) => l.trim())
      .where((l) => l.isNotEmpty)
      .toList();
  if (lines.isEmpty) fail('$name is empty');
  for (final (i, line) in lines.indexed) {
    final Object? json;
    try {
      json = jsonDecode(line);
    } on FormatException catch (e) {
      fail('$name line ${i + 1} is not JSON: $e');
    }
    if (json is! Map<String, dynamic>) {
      fail('$name line ${i + 1} is not a JSON object');
    }
    raw.add(json);
    // The app's real decoder.
    final e = JsonLinesDecoder.decodeLine(line);
    if (e is LogEvent && (e.level == 'raw' || e.level == 'debug')) {
      fail(
        '$name line ${i + 1} (type ${json['type']}) is not an event the app '
        'knows: ${e.message}',
      );
    }
    events.add(e);
  }
  return Sample(name, raw, events);
}

void main() {
  final dir = Directory(samplesDir);
  final l = lookupAppLocalizations(const Locale('en'));

  test('the engine\'s protocol samples are there', () {
    expect(
      dir.existsSync(),
      isTrue,
      reason:
          'No protocol samples: expected real engine output in '
          '${p.normalize(p.join(Directory.current.path, samplesDir))} '
          '(${expected.join(', ')}). The engine side writes them; until '
          'then this contract test fails on purpose.',
    );
    final present = dir
        .listSync()
        .whereType<File>()
        .map((f) => p.basename(f.path))
        .toSet();
    expect(present, containsAll(expected), reason: 'missing samples');
  });

  if (!dir.existsSync()) return;
  final samples = {
    for (final f in dir.listSync().whereType<File>())
      if (f.path.endsWith('.jsonl')) p.basename(f.path): f,
  };
  Sample sample(String name) {
    final f = samples[name];
    if (f == null) fail('$samplesDir/$name is missing');
    return load(f);
  }

  group('every sample', () {
    for (final name in samples.keys.toList()..sort()) {
      test('$name: every line decodes; errors have titles and end in a '
          'failed result', () {
        final s = sample(name);
        for (final e in s.of<ErrorEvent>()) {
          expect(e.code, isNotEmpty, reason: '$name: error without a code');
          expect(
            errorTitleFor(l, e.code),
            isNotNull,
            reason: '$name: no translated title for error code ${e.code}',
          );
        }
        final results = s.of<ResultEvent>();
        if (s.of<ErrorEvent>().isNotEmpty) {
          expect(results, isNotEmpty, reason: '$name: error but no result');
        }
        if (results.isNotEmpty) {
          expect(s.events.last, isA<ResultEvent>(), reason: 'result is last');
          if (s.of<ErrorEvent>().isNotEmpty) {
            expect(results.last.ok, isFalse, reason: name);
          }
        }
      });
    }
  });

  test('catalog: devices and profiles', () {
    final s = sample('catalog.jsonl');
    final cat = s.of<CatalogEvent>().single.catalog;
    final raw = s.rawOf('catalog');
    expect(cat.devices, isNotEmpty);
    expect(cat.devices, hasLength((raw['devices'] as List).length));
    expect(cat.profiles, isNotEmpty);
    expect(['windows', 'linux', 'macos', 'android'], contains(cat.host));
    for (final (i, d) in cat.devices.indexed) {
      final rd = (raw['devices'] as List)[i] as Map;
      expect(d.id, rd['id']);
      expect(d.id, isNotEmpty);
      expect(d.name, isNotEmpty, reason: d.id);
      expect(rd['support'], isA<Map>(), reason: d.id);
      expect(d.install, isNotEmpty, reason: '${d.id}: support.install');
      for (final v in d.install.values) {
        expect(['automated', 'guided', 'untested', 'none'], contains(v));
      }
    }
    expect(cat.devices.map((d) => d.id), contains('anbernic-rg-cube'));
  });

  test('resolve: connect_first, per-mode partitions, journeys, guides', () {
    final s = sample('resolve-rg-cube-heavy.jsonl');
    final plan = s.of<PlanEvent>().single.plan;
    final raw = s.rawOf('plan');
    expect(plan.device, 'anbernic-rg-cube');
    expect(plan.profile, 'heavy');

    expect(raw, contains('connect_first'));
    expect(raw['connect_first'], isA<bool>());
    expect(plan.connectFirst, raw['connect_first']);

    for (final field in ['partitions_full', 'partitions_update']) {
      expect(raw, contains(field));
      expect(raw[field], isA<List>(), reason: field);
    }
    expect(raw, contains('erases_full'));
    List<String> names(Object? v) => [
      for (final x in v as List) x is Map ? '${x['name']}' : '$x',
    ];
    expect(plan.partitionsFull, names(raw['partitions_full']));
    expect(plan.partitionsUpdate, names(raw['partitions_update']));
    expect(plan.erasesFull, names(raw['erases_full']));
    expect(plan.partitionsFull, isNotEmpty);
    expect(plan.partitionsUpdate, isNotEmpty);
    expect(plan.writtenByFull, plan.partitionsFull);
    expect(plan.writtenByUpdate, plan.partitionsUpdate);

    expect(raw['download_bytes'], isA<num>());
    expect(raw['disk_bytes'], isA<num>());
    expect(plan.diskBytes, (raw['disk_bytes'] as num).toInt());
    expect(plan.journeys.keys, containsAll(['first_time', 'returning']));
    expect(plan.journeys['first_time']!.modes, ['full']);
    expect(plan.journeys['returning']!.modes, contains('full'));
    expect(plan.guides.downloadMode, isNotEmpty);
    expect(plan.guides.downloadMode.first.title, isNotEmpty);
    expect(['automated', 'guided', 'none'], contains(plan.guides.unlockRoute));
  });

  test('space: need, free, ok and the other sizes', () {
    final s = sample('space.jsonl');
    final info = s.of<SpaceEvent>().single.space;
    final raw = s.rawOf('space');
    for (final f in ['need', 'free', 'ok', 'build_need', 'flash_need']) {
      expect(raw, contains(f), reason: f);
    }
    expect(info.need, (raw['need'] as num).toInt());
    expect(info.free, (raw['free'] as num).toInt());
    expect(info.ok, raw['ok']);
    expect(info.buildNeed, (raw['build_need'] as num).toInt());
    expect(info.flashNeed, (raw['flash_need'] as num).toInt());
    expect(info.reason, raw['reason']);
    if (raw['reason'] != null) {
      expect([
        'fat32',
        'probe_failed',
        'not_enough_space',
      ], contains(info.reason));
    }
  });

  test('flash --dry-run: stages, then a successful result', () {
    final s = sample('flash-dry-run.jsonl');
    expect(s.of<StageEvent>(), isNotEmpty);
    for (final st in s.of<StageEvent>()) {
      expect(st.title, isNotEmpty, reason: st.stage);
    }
    expect(s.of<ErrorEvent>(), isEmpty);
    expect(s.events.last, isA<ResultEvent>());
    expect((s.events.last as ResultEvent).ok, isTrue);
  });

  test('prepare: check, then prepare, then a successful result', () {
    final s = sample('prepare.jsonl');
    expect(s.of<StageEvent>().map((e) => e.stage), ['check', 'prepare']);
    expect(s.of<ErrorEvent>(), isEmpty);
    // Nothing that would ask for the device.
    expect(s.of<PromptEvent>(), isEmpty);
    expect((s.events.last as ResultEvent).ok, isTrue);
  });

  test('flash after prepare: the device prompt comes before installing', () {
    final s = sample('flash-full.jsonl');
    final stages = s.of<StageEvent>().map((e) => e.stage).toList();
    expect(stages.indexOf('wait_device'), lessThan(stages.indexOf('flash')));
    expect(s.of<PromptEvent>().single.id, 'enter_download_mode');
    // The image was checked by prepare: not read again.
    expect(
      s.of<ProgressEvent>().where((p) => p.label == 'Checking the image'),
      isEmpty,
    );
    expect((s.events.last as ResultEvent).ok, isTrue);
  });

  test('detect: device lists', () {
    final s = sample('detect.jsonl');
    final lists = s.of<DevicesEvent>();
    expect(lists, isNotEmpty);
    for (final (i, d) in lists.indexed) {
      final raw = s.raw.where((j) => j['type'] == 'devices').elementAt(i);
      expect(d.devices, hasLength((raw['devices'] as List).length));
      for (final dev in d.devices) {
        expect(dev.vidpid, matches(RegExp(r'^[0-9a-fA-F]{4}:[0-9a-fA-F]{4}$')));
        expect(dev.mode, isNotEmpty);
      }
    }
  });

  for (final (name, code) in [
    ('error-wrong-device.jsonl', 'wrong_device'),
    ('error-device-not-found.jsonl', 'device_not_found'),
  ]) {
    test('$name: $code, with a hint, then a failed result', () {
      final s = sample(name);
      final e = s.of<ErrorEvent>().single;
      expect(e.code, code);
      expect(e.message, isNotEmpty);
      expect(e.hint, isNotNull, reason: 'the hint is the notice\'s body');
      expect(errorTitleFor(l, e.code), isNotNull);
      expect((s.events.last as ResultEvent).ok, isFalse);
    });
  }

  test('drivers-failed: an error, then ok:false', () {
    final s = sample('drivers-failed.jsonl');
    expect(s.of<ErrorEvent>(), isNotEmpty);
    expect(errorTitleFor(l, s.of<ErrorEvent>().first.code), isNotNull);
    expect((s.events.last as ResultEvent).ok, isFalse);
  });

  // The samples through ProcessEngine itself (a stand-in engine that prints
  // them): one-shot answers parse, and a failed driver install throws.
  group('through ProcessEngine', () {
    late Directory tmp;
    setUp(() => tmp = Directory.systemTemp.createTempSync('thimsy-samples'));
    tearDown(() => tmp.deleteSync(recursive: true));

    ProcessEngine replaying(String name) {
      final script = File(p.join(tmp.path, 'thimsy-installer'))
        ..writeAsStringSync(
          '#!/bin/sh\ncat "${File(p.join(samplesDir, name)).absolute.path}"\n',
        );
      Process.runSync('chmod', ['+x', script.path]);
      return ProcessEngine(script.path);
    }

    test('catalog, resolve and space parse', () async {
      expect((await replaying('catalog.jsonl').catalog()).devices, isNotEmpty);
      final plan = await replaying('resolve-rg-cube-heavy.jsonl')
          .resolve(device: 'anbernic-rg-cube', profile: 'heavy');
      expect(plan.partitionsFull, isNotNull);
      final space = await replaying('space.jsonl')
          .space(device: 'anbernic-rg-cube', profile: 'heavy', workspace: '/w');
      expect(
        space.need,
        sample('space.jsonl').of<SpaceEvent>().single.space.need,
      );
    });

    test('a failed driver install throws with the engine\'s error', () async {
      await expectLater(
        replaying('drivers-failed.jsonl')
            .drivers(device: 'anbernic-rg-cube', install: true),
        throwsA(isA<EngineException>()),
      );
    });
  }, skip: Platform.isWindows ? 'the stand-in engine is a shell script' : null);
}
