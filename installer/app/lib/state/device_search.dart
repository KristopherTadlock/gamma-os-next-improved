import '../engine/engine.dart';

final _notLetterOrDigit = RegExp(r'[^\p{L}\p{N}]', unicode: true);

/// Lower case with spaces and punctuation removed, so "RG-Cube", "rg cube"
/// and "RGCube" all become "rgcube".
String normalizeForSearch(String s) =>
    s.toLowerCase().replaceAll(_notLetterOrDigit, '');

/// True when [query] finds [d] by name, maker or chip (SoC).
///
/// Case, spaces and punctuation are ignored. The whole query may match one
/// field ("rg cube" → "Anbernic RG Cube"), or each word may match some
/// field ("anbernic t820" → Anbernic devices with a Unisoc T820).
bool deviceMatches(DeviceInfo d, String query) {
  final whole = normalizeForSearch(query);
  if (whole.isEmpty) return true;
  final fields = [
    normalizeForSearch(d.name),
    normalizeForSearch(d.maker),
    normalizeForSearch(d.soc),
  ];
  if (fields.any((f) => f.contains(whole))) return true;
  final words = query
      .split(RegExp(r'\s+'))
      .map(normalizeForSearch)
      .where((w) => w.isNotEmpty)
      .toList();
  return words.length > 1 &&
      words.every((w) => fields.any((f) => f.contains(w)));
}

/// Devices sorted by maker, then name (case-insensitive).
List<DeviceInfo> sortByMaker(Iterable<DeviceInfo> devices) {
  int cmp(String a, String b) => a.toLowerCase().compareTo(b.toLowerCase());
  return devices.toList()..sort((a, b) {
    // Devices without a maker go last.
    if (a.maker.isEmpty != b.maker.isEmpty) return a.maker.isEmpty ? 1 : -1;
    final m = cmp(a.maker, b.maker);
    return m != 0 ? m : cmp(a.name, b.name);
  });
}

/// [devices] grouped by maker, makers and names sorted. Devices without a
/// maker are grouped under the empty string, last.
List<MapEntry<String, List<DeviceInfo>>> groupByMaker(
  Iterable<DeviceInfo> devices,
) {
  final groups = <String, List<DeviceInfo>>{};
  final canonical = <String, String>{};
  for (final d in sortByMaker(devices)) {
    // "ANBERNIC" and "Anbernic" are the same maker; keep the first spelling.
    final name = canonical.putIfAbsent(d.maker.toLowerCase(), () => d.maker);
    (groups[name] ??= []).add(d);
  }
  return groups.entries.toList();
}
