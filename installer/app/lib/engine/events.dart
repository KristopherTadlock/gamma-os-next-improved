import 'dart:async';
import 'dart:convert';

/// Typed view of the JSON Lines the engine prints (see
/// installer/docs/engine-protocol.md).
sealed class EngineEvent {
  const EngineEvent();

  /// Turns one decoded JSON object into an event. Unknown `type`s become a
  /// [LogEvent] so a newer engine never breaks an older app.
  static EngineEvent fromJson(Map<String, dynamic> j) {
    switch (j['type']) {
      case 'stage':
        return StageEvent(_s(j['stage']), _s(j['title'], _s(j['stage'])));
      case 'progress':
        return ProgressEvent(
          stage: _s(j['stage']),
          label: _s(j['label']),
          done: _i(j['done']),
          total: _i(j['total']),
        );
      case 'log':
        return LogEvent(_s(j['level'], 'info'), _s(j['message']));
      case 'prompt':
        return PromptEvent(
          id: _s(j['id']),
          title: _s(j['title']),
          body: _s(j['body']),
          image: j['image'] as String?,
        );
      case 'error':
        return ErrorEvent(
          code: _s(j['code'], 'internal'),
          message: _s(j['message']),
          hint: j['hint'] as String?,
        );
      case 'result':
        return ResultEvent(j['ok'] == true, j);
      case 'catalog':
        return CatalogEvent(Catalog.fromJson(j));
      case 'plan':
        return PlanEvent(Plan.fromJson(j));
      case 'space':
        return SpaceEvent(SpaceInfo.fromJson(j));
      case 'devices':
        return DevicesEvent([
          for (final d in _l(j['devices'])) DetectedDevice.fromJson(_m(d)),
        ]);
      default:
        return LogEvent('debug', 'unknown engine event: ${jsonEncode(j)}');
    }
  }
}

class StageEvent extends EngineEvent {
  final String stage;
  final String title;
  const StageEvent(this.stage, this.title);
}

class ProgressEvent extends EngineEvent {
  final String stage;
  final String label;
  final int done;
  final int total;
  const ProgressEvent({
    required this.stage,
    required this.label,
    required this.done,
    required this.total,
  });

  /// 0..1, or null when the total is unknown.
  double? get fraction =>
      total > 0 ? (done / total).clamp(0.0, 1.0).toDouble() : null;
}

class LogEvent extends EngineEvent {
  final String level;
  final String message;
  const LogEvent(this.level, this.message);
}

class PromptEvent extends EngineEvent {
  final String id;
  final String title;
  final String body;
  final String? image;
  const PromptEvent({
    required this.id,
    required this.title,
    required this.body,
    this.image,
  });
}

class ErrorEvent extends EngineEvent {
  final String code;
  final String message;
  final String? hint;

  /// True for errors the app itself raises about the engine process
  /// (`engine_crashed`, `engine_missing`, `no_answer`, `cancelled`); the UI
  /// shows its own translation for those, using [args].
  final bool local;
  final Map<String, String> args;
  const ErrorEvent({
    required this.code,
    required this.message,
    this.hint,
    this.local = false,
    this.args = const {},
  });
}

class ResultEvent extends EngineEvent {
  final bool ok;
  final Map<String, dynamic> data;
  const ResultEvent(this.ok, this.data);
  String? str(String key) => data[key] as String?;
}

class CatalogEvent extends EngineEvent {
  final Catalog catalog;
  const CatalogEvent(this.catalog);
}

class PlanEvent extends EngineEvent {
  final Plan plan;
  const PlanEvent(this.plan);
}

class SpaceEvent extends EngineEvent {
  final SpaceInfo space;
  const SpaceEvent(this.space);
}

class DevicesEvent extends EngineEvent {
  final List<DetectedDevice> devices;
  const DevicesEvent(this.devices);
}

// ---------------------------------------------------------------- models

class Catalog {
  final String version;
  final String host;
  final List<DeviceInfo> devices;
  final List<ProfileInfo> profiles;
  const Catalog({
    required this.version,
    required this.host,
    required this.devices,
    required this.profiles,
  });

  factory Catalog.fromJson(Map<String, dynamic> j) => Catalog(
    version: _s(j['version']),
    host: _s(j['host'], 'linux'),
    devices: [for (final d in _l(j['devices'])) DeviceInfo.fromJson(_m(d))],
    profiles: [for (final p in _l(j['profiles'])) ProfileInfo.fromJson(_m(p))],
  );
}

class DeviceInfo {
  final String id;
  final String name;
  final String maker;
  final String soc;
  final String display;
  final String status;
  final String build;
  final String boot;
  final Map<String, String> install;
  final List<String> notes;
  final List<String> blocks;
  final String? hostMessage;
  const DeviceInfo({
    required this.id,
    required this.name,
    required this.maker,
    required this.soc,
    required this.display,
    required this.status,
    required this.build,
    required this.boot,
    required this.install,
    this.notes = const [],
    this.blocks = const [],
    this.hostMessage,
  });

  static const defaultHostMessage =
      'This device can only be installed from a Windows PC. '
      'Run Thimsy Installer on Windows to continue.';

  /// What to tell the user when this computer can't install the device.
  String get needsWindowsMessage => hostMessage ?? defaultHostMessage;

  factory DeviceInfo.fromJson(Map<String, dynamic> j) {
    final support = _m(j['support']);
    return DeviceInfo(
      id: _s(j['id']),
      name: _s(j['name']),
      maker: _s(j['maker']),
      soc: _s(j['soc']),
      display: _s(j['display']),
      status: _s(j['status']),
      build: _s(support['build'], 'none'),
      boot: _s(support['boot'], 'none'),
      install: {
        for (final e in _m(support['install']).entries)
          e.key: e.value?.toString() ?? 'none',
      },
      notes: [for (final n in _l(j['notes'])) n.toString()],
      blocks: [for (final b in _l(j['blocks'])) b.toString()],
      hostMessage: j['host_message'] as String?,
    );
  }

  /// `automated`, `guided`, `untested` or `none` for [host].
  String installOn(String host) => install[host] ?? 'none';

  /// False only when the catalog says this computer can't install the
  /// device at all (`none`); `untested` still lets the user try.
  bool installableOn(String host) => installOn(host) != 'none';

  /// True when this computer is a tested route to a flash.
  bool worksOn(String host) {
    final s = installOn(host);
    return s == 'automated' || s == 'guided';
  }

  bool get buildVerified => build == 'verified';
}

class ProfileInfo {
  final String id;
  final String name;
  final String summary;
  final bool available;
  const ProfileInfo({
    required this.id,
    required this.name,
    required this.summary,
    required this.available,
  });
  factory ProfileInfo.fromJson(Map<String, dynamic> j) => ProfileInfo(
    id: _s(j['id']),
    name: _s(j['name']),
    summary: _s(j['summary']),
    available: j['available'] == true,
  );
}

class PlanItem {
  final String id;
  final String name;
  final String? detail;
  final bool included;
  final String? reason;
  const PlanItem({
    required this.id,
    required this.name,
    this.detail,
    this.included = true,
    this.reason,
  });
  factory PlanItem.fromJson(Map<String, dynamic> j) => PlanItem(
    id: _s(j['id']),
    name: _s(j['name'], _s(j['id'])),
    detail: (j['systems'] ?? j['system']) as String?,
    included: j['included'] != false,
    reason: j['reason'] as String?,
  );
}

class Setting {
  final String key;
  final String value;
  final String source;
  final String? reason;
  const Setting(this.key, this.value, this.source, this.reason);
  factory Setting.fromJson(Map<String, dynamic> j) => Setting(
    _s(j['key']),
    j['value']?.toString() ?? '',
    _s(j['source']),
    j['reason'] as String?,
  );
}

class GuideStep {
  final String title;
  final String body;
  final String? link;
  const GuideStep(this.title, this.body, [this.link]);
  factory GuideStep.fromJson(Map<String, dynamic> j) =>
      GuideStep(_s(j['title']), _s(j['body']), j['link'] as String?);
}

class Guides {
  final List<GuideStep> backup;
  final String unlockRoute;
  final List<GuideStep> unlock;
  final List<GuideStep> downloadMode;
  final List<GuideStep> firstBoot;
  const Guides({
    this.backup = const [],
    this.unlockRoute = 'none',
    this.unlock = const [],
    this.downloadMode = const [],
    this.firstBoot = const [],
  });

  static List<GuideStep> _steps(Object? v) => [
    for (final s in _l(v)) GuideStep.fromJson(_m(s)),
  ];

  factory Guides.fromJson(Map<String, dynamic> j) {
    final unlock = _m(j['unlock']);
    return Guides(
      backup: _steps(j['backup']),
      unlockRoute: _s(unlock['route'], 'none'),
      unlock: _steps(unlock['steps']),
      downloadMode: _steps(j['download_mode']),
      firstBoot: _steps(j['first_boot']),
    );
  }
}

/// One of the two ways into the installer: first custom-firmware install
/// (unlock, fresh install) or returning (already unlocked; update or
/// reinstall).
class Journey {
  final String id;
  final String title;
  final String body;
  final bool unlock;

  /// Allowed `flash --mode` values, default first.
  final List<String> modes;
  final String? updateNote;
  const Journey({
    required this.id,
    required this.title,
    required this.body,
    required this.unlock,
    required this.modes,
    this.updateNote,
  });

  factory Journey.fromJson(String id, Map<String, dynamic> j) => Journey(
    id: id,
    title: _s(j['title']),
    body: _s(j['body']),
    unlock: j['unlock'] == true,
    modes: [for (final m in _l(j['modes'])) m.toString()],
    updateNote: j['update_note'] as String?,
  );

  /// Used when an engine's plan has no `journeys`. Titles and texts are
  /// empty so the app shows its own translations.
  static const defaults = {
    'first_time': Journey(
      id: 'first_time',
      title: '',
      body: '',
      unlock: true,
      modes: ['full'],
    ),
    'returning': Journey(
      id: 'returning',
      title: '',
      body: '',
      unlock: false,
      modes: ['update', 'full'],
    ),
  };
}

class Plan {
  final String device;
  final String profile;
  final bool available;
  final String? unavailableReason;
  final List<PlanItem> emulators;
  final List<PlanItem> cores;
  final List<PlanItem> apps;
  final List<PlanItem> excluded;
  final List<Setting> settings;
  final int downloadBytes;
  final int diskBytes;
  final List<String> partitions;
  final bool wipesData;
  final Guides guides;
  final Map<String, Journey> journeys;
  const Plan({
    required this.device,
    required this.profile,
    required this.available,
    this.unavailableReason,
    this.emulators = const [],
    this.cores = const [],
    this.apps = const [],
    this.excluded = const [],
    this.settings = const [],
    this.downloadBytes = 0,
    this.diskBytes = 0,
    this.partitions = const [],
    this.wipesData = true,
    this.guides = const Guides(),
    this.journeys = Journey.defaults,
  });

  factory Plan.fromJson(Map<String, dynamic> j) {
    List<PlanItem> items(Object? v) => [
      for (final i in _l(v)) PlanItem.fromJson(_m(i)),
    ];
    return Plan(
      device: _s(j['device']),
      profile: _s(j['profile']),
      available: j['available'] != false,
      unavailableReason: j['unavailable_reason'] as String?,
      emulators: items(j['emulators']),
      cores: items(j['cores']),
      apps: items(j['apps']),
      excluded: items(j['excluded']),
      settings: [for (final s in _l(j['settings'])) Setting.fromJson(_m(s))],
      downloadBytes: _i(j['download_bytes']),
      diskBytes: _i(j['disk_bytes']),
      partitions: [for (final p in _l(j['partitions'])) p.toString()],
      wipesData: j['wipes_data'] != false,
      guides: Guides.fromJson(_m(j['guides'])),
      journeys: j['journeys'] is Map && (j['journeys'] as Map).isNotEmpty
          ? {
              for (final e in _m(j['journeys']).entries)
                e.key: Journey.fromJson(e.key, _m(e.value)),
            }
          : Journey.defaults,
    );
  }
}

/// Answer of `space`: how much the build (and flash) needs and how much the
/// workspace drive has.
class SpaceInfo {
  final int need;
  final int free;
  final bool ok;
  final int buildNeed;
  final int flashNeed;
  final int cached;
  final int downloads;
  const SpaceInfo({
    required this.need,
    required this.free,
    required this.ok,
    this.buildNeed = 0,
    this.flashNeed = 0,
    this.cached = 0,
    this.downloads = 0,
  });
  factory SpaceInfo.fromJson(Map<String, dynamic> j) => SpaceInfo(
    need: _i(j['need']),
    free: _i(j['free']),
    ok: j['ok'] == true,
    buildNeed: _i(j['build_need']),
    flashNeed: _i(j['flash_need']),
    cached: _i(j['cached']),
    downloads: _i(j['downloads']),
  );
}

/// One removable thing in the workspace (from `clean --dry-run`).
class CleanItem {
  /// `downloads`, `image`, `backup` or `scratch`.
  final String kind;
  final String path;
  final int bytes;
  const CleanItem(this.kind, this.path, this.bytes);
  factory CleanItem.fromJson(Map<String, dynamic> j) =>
      CleanItem(_s(j['kind']), _s(j['path']), _i(j['bytes']));

  static List<CleanItem> listFrom(ResultEvent r) => [
    for (final i in _l(r.data['items'])) CleanItem.fromJson(_m(i)),
  ];
}

class DetectedDevice {
  final String vidpid;
  final String mode;
  final String? device;
  final String label;
  const DetectedDevice({
    required this.vidpid,
    required this.mode,
    this.device,
    required this.label,
  });
  factory DetectedDevice.fromJson(Map<String, dynamic> j) => DetectedDevice(
    vidpid: _s(j['vidpid']),
    mode: _s(j['mode'], 'unknown'),
    device: j['device'] as String?,
    label: _s(j['label'], _s(j['vidpid'])),
  );
}

// ---------------------------------------------------------- JSON Lines

/// Splits a byte-decoded text stream into lines and turns each line into an
/// [EngineEvent]. Lines that aren't JSON objects become [LogEvent]s, so a
/// stray print from a tool never kills the run.
class JsonLinesDecoder extends StreamTransformerBase<String, EngineEvent> {
  const JsonLinesDecoder();

  static EngineEvent decodeLine(String line) {
    try {
      final v = jsonDecode(line);
      if (v is Map<String, dynamic>) return EngineEvent.fromJson(v);
    } on FormatException {
      // fall through
    } on TypeError {
      // fall through
    }
    return LogEvent('raw', line);
  }

  @override
  Stream<EngineEvent> bind(Stream<String> stream) async* {
    await for (final line in stream.transform(const LineSplitter())) {
      final t = line.trim();
      if (t.isEmpty) continue;
      yield decodeLine(t);
    }
  }
}

// --------------------------------------------------------------- helpers

String _s(Object? v, [String fallback = '']) =>
    v == null ? fallback : v.toString();
int _i(Object? v) => v is num ? v.toInt() : int.tryParse('$v') ?? 0;
List<Object?> _l(Object? v) => v is List ? v : const [];
Map<String, dynamic> _m(Object? v) =>
    v is Map ? v.map((k, val) => MapEntry('$k', val)) : const {};
