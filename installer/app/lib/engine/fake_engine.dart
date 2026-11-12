import 'dart:async';

import 'engine.dart';

/// Canned engine for widget tests and `--dart-define=FAKE_ENGINE=true`.
/// Answers in the same shapes as the real engine, after [step] per event.
class FakeEngine extends Engine {
  final Duration step;
  final String host;

  /// Device id the fake `detect` reports as plugged in (download mode).
  final String detectDevice;

  /// Calls made, for assertions in tests.
  final List<String> calls = [];

  /// Free bytes the fake `space` reports (tests change it to "free up").
  int freeBytes;

  /// How many times `flash` fails with `not_enough_space` before working.
  int flashSpaceFailures;

  /// What `clean` would remove, by kind.
  final Map<String, int> cleanable = {
    'downloads': 2680000000,
    'image': 7722216910,
    'backup': 68157440,
    'scratch': 5900000000,
  };

  static const buildNeed = 19000000000;
  static const flashNeed = 7722216910;
  static const downloadsBytes = 2680000000;

  FakeEngine({
    this.step = const Duration(milliseconds: 250),
    this.host = 'linux',
    this.detectDevice = 'anbernic-rg-cube',
    this.freeBytes = 120000000000,
    this.flashSpaceFailures = 0,
  });

  static const _rgCube = {
    'id': 'anbernic-rg-cube',
    'name': 'Anbernic RG Cube',
    'maker': 'Anbernic',
    'soc': 'Unisoc T820',
    'display': '720×720',
    'status': 'testing',
    'support': {
      'build': 'verified',
      'boot': 'qualified',
      'install': {
        'windows': 'automated',
        'linux': 'automated',
        'macos': 'untested',
        'android': 'none',
      },
    },
    'notes': ['Thimsy is built for this device first.'],
    'blocks': <String>[],
    'host_message': null,
  };

  static const _rp4 = {
    'id': 'retroid-pocket-4-pro',
    'name': 'Retroid Pocket 4 Pro',
    'maker': 'Retroid',
    'soc': 'MediaTek Dimensity 1100',
    'display': '1334×750',
    'status': 'candidate',
    'support': {
      'build': 'none',
      'boot': 'none',
      'install': {
        'windows': 'automated',
        'linux': 'automated',
        'macos': 'automated',
        'android': 'none',
      },
    },
    'notes': ['Not yet supported.'],
    'blocks': <String>[],
  };

  static const _rg557 = {
    'id': 'anbernic-rg557',
    'name': 'Anbernic RG557',
    'maker': 'Anbernic',
    'soc': 'MediaTek Dimensity 8300',
    'display': '1920×1080',
    'status': 'testing',
    'support': {
      'build': 'verified',
      'boot': 'none',
      'install': {
        'windows': 'automated',
        'linux': 'automated',
        'macos': 'none',
        'android': 'none',
      },
    },
    'notes': ['Don\'t install if the stock firmware is v1.26.'],
    'blocks': <String>[],
    'host_message':
        'The RG557 is flashed with SP Flash Tool, which runs on Windows and '
        'Linux only. Run Thimsy Installer on a Windows PC to continue.',
  };

  static const _e6 = {
    'id': 'gamemt-e6-max',
    'name': 'GameMT E6 MAX',
    'maker': 'GameMT',
    'soc': 'Allwinner A527',
    'display': '1280×720',
    'status': 'candidate',
    'support': {
      'build': 'none',
      'boot': 'none',
      'install': {
        'windows': 'guided',
        'linux': 'none',
        'macos': 'none',
        'android': 'none',
      },
    },
    'notes': <String>[],
    'blocks': <String>[],
  };

  Map<String, dynamic> get _catalogJson => {
    'type': 'catalog',
    'version': '0.1.0-fake',
    'host': host,
    'devices': [_rgCube, _rp4, _rg557, _e6],
    'profiles': [
      {
        'id': 'light',
        'name': 'Light',
        'summary': 'Classic systems up to PlayStation, smallest download.',
        'available': false,
      },
      {
        'id': 'medium',
        'name': 'Medium',
        'summary': 'Balanced: adds PSP, Dreamcast, N64 and DS.',
        'available': true,
      },
      {
        'id': 'heavy',
        'name': 'Heavy',
        'summary': 'Adds GameCube, 3DS, PS2, Vita and Xbox.',
        'available': false,
      },
    ],
  };

  Future<void> _tick() => Future<void>.delayed(step);

  Map<String, dynamic> _device(String id) => (_catalogJson['devices'] as List)
      .cast<Map<String, dynamic>>()
      .firstWhere((d) => d['id'] == id, orElse: () => _rgCube);

  @override
  Future<Catalog> catalog() async {
    calls.add('catalog');
    await _tick();
    return Catalog.fromJson(_catalogJson);
  }

  @override
  Future<Plan> resolve({
    required String device,
    required String profile,
    Map<String, String> overrides = const {},
  }) async {
    calls.add('resolve $device $profile');
    await _tick();
    final available = profile == 'medium';
    final automatedUnlock = device == 'retroid-pocket-4-pro';
    return Plan.fromJson({
      'type': 'plan',
      'device': device,
      'profile': profile,
      'available': available,
      'unavailable_reason': available ? null : 'Not published yet',
      'emulators': [
        {
          'id': 'retroarch',
          'name': 'RetroArch',
          'systems': '8- and 16-bit, PS1',
        },
        {'id': 'ppsspp', 'name': 'PPSSPP', 'systems': 'PSP'},
        {'id': 'flycast', 'name': 'Flycast', 'systems': 'Dreamcast'},
        {'id': 'watermelonds', 'name': 'WatermelonDS', 'systems': 'DS'},
      ],
      'cores': [
        {'id': 'mgba', 'system': 'Game Boy Advance', 'default': true},
        {'id': 'snes9x', 'system': 'SNES', 'default': true},
      ],
      'apps': [
        {'id': 'brave', 'name': 'Brave'},
        {'id': 'aurora', 'name': 'Aurora Store'},
      ],
      'excluded': [
        {'id': 'armsx2', 'name': 'ARMSX2', 'reason': 'Heavy profile only'},
      ],
      'settings': [
        {'key': 'nano.theme', 'value': 'esde', 'source': 'profile'},
      ],
      'download_bytes': 4800000000,
      'disk_bytes': 19000000000,
      'partitions': ['super', 'vendor_boot_a', 'vendor_boot_b', 'logo'],
      'wipes_data': true,
      'journeys': {
        'first_time': {
          'title': 'First time installing custom firmware',
          'body':
              'Your device still runs its original firmware. We\'ll '
              'unlock it first; installing erases it.',
          'unlock': true,
          'modes': ['full'],
        },
        'returning': {
          'title': 'Already unlocked / installing an update',
          'body': 'Your device is unlocked and already runs custom firmware.',
          'unlock': false,
          'modes': ['update', 'full'],
          'update_note':
              'Keeps your data. Only when the device already runs '
              'GammaOS Next 1.2-based firmware or Thimsy.',
        },
      },
      'guides': {
        'backup': [
          {
            'title': 'Copy your saves and games',
            'body':
                'Installing erases internal storage. Copy ROMs, saves and '
                'screenshots to your computer or the microSD card.',
          },
        ],
        'unlock': {
          'route': automatedUnlock ? 'automated' : 'guided',
          'steps': [
            {
              'title': 'Follow the upstream unlock steps',
              'body': 'GammaOS documents how to unlock this device.',
              'link': 'https://github.com/TheGammaSqueeze/GammaOSNext/wiki/GammaOS-Next-Installation',
            },
          ],
        },
        'download_mode': [
          {
            'title': 'Enter download mode',
            'body': 'Power the device off. Hold Vol− and plug in the cable.',
          },
        ],
        'first_boot': [
          {
            'title': 'Nano home appears',
            'body': 'The first boot takes a few minutes.',
          },
          {
            'title': 'Controls and touch work',
            'body': 'Try the D-pad and a tap.',
          },
        ],
      },
    });
  }

  EngineRun _script(List<EngineEvent> Function() events) => _FakeRun(() async* {
    for (final e in events()) {
      await _tick();
      yield e;
    }
  });

  ErrorEvent? _hostError(String device) {
    final d = DeviceInfo.fromJson(_device(device));
    if (d.installableOn(host)) return null;
    return ErrorEvent(
      code: 'unsupported_host',
      message: d.needsWindowsMessage,
      hint: 'Use a Windows PC',
    );
  }

  @override
  Future<SpaceInfo> space({
    required String device,
    required String profile,
    required String workspace,
    bool deleteDownloads = false,
  }) async {
    calls.add('space $workspace${deleteDownloads ? ' delete' : ''}');
    await _tick();
    final need = buildNeed - (deleteDownloads ? downloadsBytes : 0);
    return SpaceInfo(
      need: need,
      free: freeBytes,
      ok: freeBytes >= need,
      buildNeed: need,
      flashNeed: flashNeed,
      downloads: downloadsBytes,
    );
  }

  @override
  Future<ResultEvent> clean({
    required String workspace,
    bool dryRun = false,
    bool downloads = false,
    bool image = false,
    bool scratch = false,
    bool backup = false,
  }) async {
    final kinds = {
      if (downloads) 'downloads',
      if (image) 'image',
      if (scratch) 'scratch',
      if (backup) 'backup',
    };
    calls.add(
      'clean${dryRun ? ' dry' : ''} ${(kinds.toList()..sort()).join(',')}'
          .trim(),
    );
    await _tick();
    if (dryRun) {
      return ResultEvent(true, {
        'type': 'result',
        'ok': true,
        'items': [
          for (final e in cleanable.entries)
            {'kind': e.key, 'path': '$workspace/${e.key}', 'bytes': e.value},
        ],
        'freed': 0,
      });
    }
    var freed = 0;
    for (final k in kinds) {
      freed += cleanable.remove(k) ?? 0;
    }
    return ResultEvent(true, {
      'type': 'result',
      'ok': true,
      'items': const [],
      'freed': freed,
    });
  }

  @override
  EngineRun build({
    required String device,
    required String profile,
    required String workspace,
    String? importDir,
    bool deleteDownloads = false,
    Map<String, String> overrides = const {},
  }) {
    calls.add('build $device $profile${deleteDownloads ? ' delete' : ''}');
    final job = '$workspace/jobs/$device-$profile';
    return _script(
      () => [
        const StageEvent('plan', 'Planning'),
        const StageEvent('fetch', 'Downloading'),
        for (var i = 1; i <= 4; i++)
          ProgressEvent(
            stage: 'fetch',
            label: 'RG_Cube_GammaOS_Next_Full_v1.2.7z.00$i',
            done: i * 25,
            total: 100,
          ),
        const StageEvent('extract', 'Extracting'),
        const ProgressEvent(stage: 'extract', label: 'PAC', done: 1, total: 1),
        const StageEvent('assemble', 'Building the Thimsy image'),
        const LogEvent('info', 'lp: replace system_a'),
        const StageEvent('validate', 'Checking the image'),
        const StageEvent('publish', 'Finishing'),
        ResultEvent(true, {
          'type': 'result',
          'ok': true,
          'job': job,
          'image': '$job/out/Thimsy.pac',
          'bytes': 7722216910,
        }),
      ],
    );
  }

  @override
  EngineRun detect() {
    calls.add('detect');
    return _FakeRun(() async* {
      await _tick();
      yield const DevicesEvent([]);
      await _tick();
      yield DevicesEvent([
        DetectedDevice(
          vidpid: '1782:4d00',
          mode: 'download',
          device: detectDevice,
          label: 'Unisoc download mode',
        ),
      ]);
      // Keeps watching until cancelled, like the real `detect --watch`.
    }, keepOpen: true);
  }

  @override
  EngineRun flash({
    required String job,
    bool dryRun = false,
    FlashMode mode = FlashMode.full,
  }) {
    calls.add('flash ${mode.name}${dryRun ? ' dry' : ''}');
    final device = _deviceFromJob(job);
    if (flashSpaceFailures > 0) {
      flashSpaceFailures--;
      return _script(
        () => [
          const StageEvent('check', 'Checking the image'),
          const ErrorEvent(
            code: 'not_enough_space',
            message:
                'Not enough free space to prepare the partitions: needs '
                '7.7 GB, 2.0 GB available.',
            hint: 'Free up space on the workspace drive.',
          ),
          ResultEvent(false, {'type': 'result', 'ok': false}),
        ],
      );
    }
    final hostErr = _hostError(device);
    if (hostErr != null) {
      return _script(
        () => [
          hostErr,
          ResultEvent(false, {'type': 'result', 'ok': false}),
        ],
      );
    }
    return _script(
      () => [
        const StageEvent('check', 'Checking the image'),
        const StageEvent('driver', 'Checking the USB driver'),
        if (mode == FlashMode.full)
          const StageEvent('backup', 'Backing up calibration'),
        const StageEvent('wait_device', 'Waiting for the device'),
        const PromptEvent(
          id: 'enter_download_mode',
          title: 'Hold Vol− and plug in the cable now',
          body: 'Keep holding until the progress bar moves.',
        ),
        const StageEvent('flash', 'Writing Thimsy'),
        for (var i = 1; i <= 4; i++)
          ProgressEvent(stage: 'flash', label: 'super', done: i, total: 4),
        const StageEvent('reboot', 'Restarting'),
        ResultEvent(true, {
          'type': 'result',
          'ok': true,
          'backup': '$job/backup',
        }),
      ],
    );
  }

  String _deviceFromJob(String job) {
    final name = job.split(RegExp(r'[\\/]')).last;
    for (final d in (_catalogJson['devices'] as List)) {
      final id = (d as Map)['id'] as String;
      if (name.startsWith(id)) return id;
    }
    return 'anbernic-rg-cube';
  }

  @override
  EngineRun unlock({required String device}) {
    calls.add('unlock $device');
    return _script(
      () => [
        const StageEvent('unlock', 'Unlocking'),
        const PromptEvent(
          id: 'confirm_unlock',
          title: 'Press Volume Up on the handheld',
          body: 'This confirms the unlock on the device.',
        ),
        ResultEvent(true, {'type': 'result', 'ok': true}),
      ],
    );
  }

  @override
  Future<ResultEvent> drivers({
    required String device,
    bool install = false,
  }) async {
    calls.add('drivers $device${install ? ' install' : ''}');
    await _tick();
    final status = host != 'windows'
        ? 'not_required'
        : (install ? 'ok' : 'missing');
    return ResultEvent(true, {
      'type': 'result',
      'ok': true,
      'status': status,
      'detail': status == 'missing'
          ? 'WinUSB is not installed for 1782:4d00'
          : '',
    });
  }
}

class _FakeRun implements EngineRun {
  final StreamController<EngineEvent> _out = StreamController();
  StreamSubscription<EngineEvent>? _sub;
  bool _closed = false;

  _FakeRun(Stream<EngineEvent> Function() source, {bool keepOpen = false}) {
    _sub = source().listen(
      _out.add,
      onDone: () {
        if (!keepOpen) _close();
      },
    );
  }

  void _close() {
    if (_closed) return;
    _closed = true;
    _out.close();
  }

  @override
  Stream<EngineEvent> get events => _out.stream;

  @override
  Future<void> cancel() async {
    await _sub?.cancel();
    _close();
  }
}
