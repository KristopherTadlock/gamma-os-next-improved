import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:thimsy_installer/engine/events.dart';

Future<List<EngineEvent>> decode(List<String> chunks) =>
    Stream.fromIterable(chunks).transform(const JsonLinesDecoder()).toList();

void main() {
  group('JsonLinesDecoder', () {
    test('joins a line split across chunks', () async {
      final events = await decode([
        '{"type":"stage","stage":"fetch","ti',
        'tle":"Downloading"}\n',
      ]);
      expect(events, hasLength(1));
      final e = events.single as StageEvent;
      expect(e.stage, 'fetch');
      expect(e.title, 'Downloading');
    });

    test('splits several events in one chunk', () async {
      final events = await decode([
        '{"type":"log","level":"info","message":"a"}\n'
            '{"type":"progress","stage":"fetch","label":"x","done":5,"total":10}\n'
            '{"type":"result","ok":true,"job":"/w/jobs/j"}',
      ]);
      expect(events.map((e) => e.runtimeType).toList(), [
        LogEvent,
        ProgressEvent,
        ResultEvent,
      ]);
      expect((events[1] as ProgressEvent).fraction, 0.5);
      expect((events[2] as ResultEvent).str('job'), '/w/jobs/j');
    });

    test('turns a malformed line into a log event', () async {
      final events = await decode(['not json at all\n', '[1,2]\n', '\n']);
      expect(events, hasLength(2));
      expect(events.every((e) => e is LogEvent), isTrue);
      expect((events.first as LogEvent).message, 'not json at all');
    });

    test('handles CRLF line endings', () async {
      final events = await decode(['{"type":"log","message":"w"}\r\n']);
      expect((events.single as LogEvent).message, 'w');
    });

    test('unknown event types become debug logs', () async {
      final events = await decode(['{"type":"future_thing","x":1}\n']);
      expect((events.single as LogEvent).level, 'debug');
    });
  });

  group('models', () {
    test('parses a catalog with host support and host_message', () {
      final e = EngineEvent.fromJson({
        'type': 'catalog',
        'version': '0.1.0',
        'host': 'macos',
        'devices': [
          {
            'id': 'anbernic-rg557',
            'name': 'Anbernic RG557',
            'maker': 'Anbernic',
            'soc': 'MediaTek Dimensity 8300',
            'display': '1920×1080',
            'status': 'testing',
            'support': {
              'build': 'verified',
              'boot': 'none',
              'install': {'windows': 'automated', 'macos': 'none'},
            },
            'host_message': 'Use Windows for this one.',
          },
          {
            'id': 'x',
            'name': 'X',
            'support': {
              'install': {'macos': 'untested'},
            },
          },
        ],
        'profiles': [
          {'id': 'medium', 'name': 'Medium', 'summary': 's', 'available': true},
        ],
      });
      final cat = (e as CatalogEvent).catalog;
      expect(cat.host, 'macos');
      final rg = cat.devices.first;
      expect(rg.installableOn('macos'), isFalse);
      expect(rg.installableOn('windows'), isTrue);
      expect(rg.buildVerified, isTrue);
      expect(rg.needsWindowsMessage, 'Use Windows for this one.');
      final x = cat.devices.last;
      expect(x.installableOn('macos'), isTrue);
      expect(x.worksOn('macos'), isFalse);
      expect(x.needsWindowsMessage, DeviceInfo.defaultHostMessage);
      expect(cat.profiles.single.available, isTrue);
    });

    test('parses a plan with guides', () {
      final e = EngineEvent.fromJson({
        'type': 'plan',
        'device': 'd',
        'profile': 'medium',
        'available': true,
        'emulators': [
          {'id': 'ppsspp', 'name': 'PPSSPP', 'systems': 'PSP'},
        ],
        'excluded': [
          {'id': 'armsx2', 'name': 'ARMSX2', 'reason': 'needs ARMv8.2'},
        ],
        'download_bytes': 10,
        'disk_bytes': 20,
        'partitions': ['super'],
        'wipes_data': false,
        'guides': {
          'backup': [
            {'title': 't', 'body': 'b'},
          ],
          'unlock': {
            'route': 'automated',
            'steps': [
              {'title': 'u', 'body': 'v', 'link': 'https://x'},
            ],
          },
        },
      });
      final plan = (e as PlanEvent).plan;
      expect(plan.emulators.single.detail, 'PSP');
      expect(plan.excluded.single.reason, 'needs ARMv8.2');
      expect(plan.wipesData, isFalse);
      expect(plan.guides.unlockRoute, 'automated');
      expect(plan.guides.unlock.single.link, 'https://x');
      expect(plan.guides.firstBoot, isEmpty);
    });
  });
}
