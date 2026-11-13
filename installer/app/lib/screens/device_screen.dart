import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../engine/engine.dart';
import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../state/device_search.dart';
import '../widgets/common.dart';

/// Host operating system names are product names: not translated.
const hostNames = {
  'windows': 'Windows',
  'macos': 'macOS',
  'linux': 'Linux',
  'android': 'Android',
};

/// Narrowest a device card gets before the grid drops a column.
const _minCardWidth = 220.0;
const _cardGap = 16.0;

class DeviceScreen extends StatefulWidget {
  const DeviceScreen({super.key});

  @override
  State<DeviceScreen> createState() => _DeviceScreenState();
}

class _DeviceScreenState extends State<DeviceScreen> {
  final _search = TextEditingController();
  final _cardKeys = <String, GlobalKey>{};

  /// Bumped when the device is picked outside the dropdown, so the
  /// dropdown starts fresh showing that device (and no half-typed filter).
  int _dropdownGeneration = 0;

  // The dropdown's entries, rebuilt only when the device list changes.
  List<DropdownMenuEntry<String>> _entries = const [];
  Map<String, DeviceInfo> _byId = const {};
  Catalog? _entriesCatalog;
  bool? _entriesWorksHere;

  @override
  void initState() {
    super.initState();
    _search.addListener(_onSearchChanged);
  }

  @override
  void dispose() {
    _search.removeListener(_onSearchChanged);
    _search.dispose();
    super.dispose();
  }

  void _onSearchChanged() => setState(() {});

  void _updateEntries(AppState s) {
    if (identical(_entriesCatalog, s.catalog) &&
        _entriesWorksHere == s.worksHereOnly) {
      return;
    }
    _entriesCatalog = s.catalog;
    _entriesWorksHere = s.worksHereOnly;
    final devices = sortByMaker(s.visibleDevices);
    _byId = {for (final d in devices) d.id: d};
    _entries = [
      for (final d in devices) DropdownMenuEntry(value: d.id, label: d.name),
    ];
  }

  /// Picked by tapping a card or pressing Enter in the search box.
  void _pickFromList(AppState s, String id) {
    s.selectDevice(id);
    setState(() => _dropdownGeneration++);
  }

  /// Picked in the dropdown: make sure its card is shown and in view.
  void _pickFromDropdown(AppState s, String? id) {
    if (id == null) return;
    final d = _byId[id];
    if (d != null && !deviceMatches(d, _search.text)) _search.clear();
    s.selectDevice(id);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final ctx = _cardKeys[id]?.currentContext;
      if (ctx != null && ctx.mounted) {
        Scrollable.ensureVisible(
          ctx,
          alignment: 0.2,
          duration: const Duration(milliseconds: 250),
        );
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final hostName = hostNames[s.host] ?? s.host;
    if (s.catalogError != null) {
      return StepPage(
        title: l.deviceTitle,
        children: [
          ErrorNotice(
            title: l.deviceListError,
            error: errorEventOf(s.catalogError!),
            action: OutlinedButton(
              onPressed: s.loadCatalog,
              child: Text(l.tryAgain),
            ),
          ),
        ],
      );
    }
    if (s.catalog == null) {
      return const Center(child: CircularProgressIndicator());
    }
    _updateEntries(s);
    final t = Theme.of(context).textTheme;
    final shown = [
      for (final d in s.visibleDevices)
        if (deviceMatches(d, _search.text)) d,
    ];

    return LayoutBuilder(
      builder: (context, box) {
        const pad = EdgeInsetsDirectional.fromSTEB(32, 28, 32, 24);
        final width = box.maxWidth - pad.horizontal;
        final header = [
          Text(l.deviceTitle, style: t.headlineSmall),
          const SizedBox(height: 6),
          Text(l.runningOn(hostName), style: t.bodyMedium),
          const SizedBox(height: 20),
          _controls(l, s, width),
          const SizedBox(height: 4),
          SwitchListTile(
            key: const ValueKey('works-here'),
            contentPadding: EdgeInsets.zero,
            title: Text(l.onlyInstallable),
            value: s.worksHereOnly,
            onChanged: (v) {
              s.setWorksHereOnly(v);
              setState(() => _dropdownGeneration++);
            },
          ),
          const SizedBox(height: 8),
        ];
        final list = _deviceList(l, s, shown, hostName, width);
        final details = _details(s);

        // Short windows: everything scrolls together.
        if (box.maxHeight < 560) {
          return ListView(
            padding: pad,
            children: [
              ...header,
              list,
              if (details != null) ...[const SizedBox(height: 20), details],
            ],
          );
        }
        // Otherwise the search box stays at the top and the selected
        // device's notes stay at the bottom; only the list scrolls.
        return Padding(
          padding: pad.copyWith(bottom: 0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              ...header,
              Expanded(
                child: SingleChildScrollView(
                  key: const ValueKey('device-list'),
                  padding: const EdgeInsets.only(bottom: 16),
                  child: list,
                ),
              ),
              if (details != null) ...[
                const Divider(height: 1),
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 180),
                  child: SingleChildScrollView(
                    padding: const EdgeInsets.only(top: 12, bottom: 12),
                    child: details,
                  ),
                ),
              ],
            ],
          ),
        );
      },
    );
  }

  /// The search box and the dropdown: side by side, or stacked when narrow.
  Widget _controls(AppLocalizations l, AppState s, double width) {
    final search = TextField(
      key: const ValueKey('device-search'),
      controller: _search,
      decoration: InputDecoration(
        prefixIcon: const Icon(Icons.search),
        hintText: l.deviceSearchHint,
        border: const OutlineInputBorder(),
        suffixIcon: _search.text.isEmpty
            ? null
            : IconButton(
                key: const ValueKey('device-search-clear'),
                tooltip: l.deviceSearchClear,
                icon: const Icon(Icons.close),
                onPressed: _search.clear,
              ),
      ),
      onSubmitted: (q) {
        final hits = [
          for (final d in s.visibleDevices)
            if (deviceMatches(d, q)) d,
        ];
        if (q.trim().isNotEmpty && hits.length == 1) {
          _pickFromList(s, hits.single.id);
        }
      },
    );
    final dropdown = KeyedSubtree(
      key: ValueKey('device-dropdown-$_dropdownGeneration'),
      child: DropdownMenu<String>(
        key: const ValueKey('device-dropdown'),
        // Fills its parent; also keeps the hidden width-measuring copy of
        // the menu out of the tree.
        expandedInsets: EdgeInsets.zero,
        label: Text(l.devicePickerLabel),
        leadingIcon: const Icon(Icons.videogame_asset_outlined),
        initialSelection: s.deviceId,
        enableFilter: true,
        enableSearch: true,
        requestFocusOnTap: true,
        menuHeight: 360,
        dropdownMenuEntries: _entries,
        filterCallback: (entries, filter) => [
          for (final e in entries)
            if (_byId[e.value] case final d? when deviceMatches(d, filter)) e,
        ],
        searchCallback: (entries, query) {
          final q = normalizeForSearch(query);
          if (q.isEmpty) return null;
          final exact = entries.indexWhere(
            (e) => normalizeForSearch(e.label) == q,
          );
          if (exact != -1) return exact;
          final i = entries.indexWhere(
            (e) =>
                _byId[e.value] != null && deviceMatches(_byId[e.value]!, query),
          );
          return i == -1 ? null : i;
        },
        onSelected: (id) => _pickFromDropdown(s, id),
      ),
    );
    if (width >= 640) {
      return Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(child: search),
          const SizedBox(width: 16),
          SizedBox(width: width >= 900 ? 340 : 280, child: dropdown),
        ],
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [search, const SizedBox(height: 12), dropdown],
    );
  }

  /// Devices grouped by maker under small headings, or the empty state.
  Widget _deviceList(
    AppLocalizations l,
    AppState s,
    List<DeviceInfo> shown,
    String hostName,
    double width,
  ) {
    if (shown.isEmpty) return _noResults(l, s);
    final columns = ((width + _cardGap) / (_minCardWidth + _cardGap))
        .floor()
        .clamp(1, 6);
    final cardWidth = (width - _cardGap * (columns - 1)) / columns;
    final groups = groupByMaker(shown);
    final scheme = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final (i, g) in groups.indexed) ...[
          Padding(
            padding: EdgeInsets.only(top: i == 0 ? 4 : 24, bottom: 10),
            child: Row(
              key: ValueKey('maker-${g.key}'),
              children: [
                Text(
                  g.key.isEmpty ? l.deviceMakerOther : g.key,
                  style: Theme.of(context).textTheme.titleSmall
                      ?.copyWith(color: scheme.primary),
                ),
                const SizedBox(width: 12),
                const Expanded(child: Divider()),
              ],
            ),
          ),
          Wrap(
            spacing: _cardGap,
            runSpacing: _cardGap,
            children: [
              for (final d in g.value)
                KeyedSubtree(
                  key: _cardKeys.putIfAbsent(d.id, GlobalKey.new),
                  child: _DeviceCard(
                    device: d,
                    host: s.host,
                    hostName: hostName,
                    width: cardWidth,
                    selected: d.id == s.deviceId,
                    onTap: () => _pickFromList(s, d.id),
                  ),
                ),
            ],
          ),
        ],
      ],
    );
  }

  Widget _noResults(AppLocalizations l, AppState s) {
    final t = Theme.of(context).textTheme;
    final searching = _search.text.trim().isNotEmpty;
    return Padding(
      key: const ValueKey('device-no-results'),
      padding: const EdgeInsets.symmetric(vertical: 32, horizontal: 16),
      child: Column(
        children: [
          Icon(
            Icons.search_off,
            size: 48,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 12),
          Text(
            searching
                ? l.deviceNoMatchTitle(_search.text.trim())
                : l.deviceNoneHereTitle,
            style: t.titleMedium,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 4),
          Text(
            searching ? l.deviceNoMatchBody : l.deviceNoneHereBody,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 12),
          if (searching)
            OutlinedButton.icon(
              key: const ValueKey('device-no-results-clear'),
              icon: const Icon(Icons.close),
              label: Text(l.deviceSearchClear),
              onPressed: _search.clear,
            )
          else
            OutlinedButton(
              key: const ValueKey('device-show-all'),
              onPressed: () {
                s.setWorksHereOnly(false);
                setState(() => _dropdownGeneration++);
              },
              child: Text(l.deviceShowAll),
            ),
        ],
      ),
    );
  }

  /// The selected device's catalog notes and blocks, if any.
  Widget? _details(AppState s) {
    final d = s.device;
    if (d == null || (d.notes.isEmpty && d.blocks.isEmpty)) return null;
    return Column(
      key: const ValueKey('device-details'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final n in d.notes)
          Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: Text('• $n'),
          ),
        for (final b in d.blocks)
          Notice(
            icon: Icons.block,
            title: b,
            color: Theme.of(context).colorScheme.error,
          ),
      ],
    );
  }
}

class _DeviceCard extends StatelessWidget {
  final DeviceInfo device;
  final String host;
  final String hostName;
  final double width;
  final bool selected;
  final VoidCallback onTap;
  const _DeviceCard({
    required this.device,
    required this.host,
    required this.hostName,
    required this.width,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final support = device.installOn(host);
    final needsWindows = !device.installableOn(host);
    final badge = needsWindows
        ? _Badge(
            key: ValueKey('needs-windows-${device.id}'),
            icon: Icons.warning_amber,
            text: l.badgeNeedsWindows,
            color: scheme.error,
          )
        : _Badge(
            icon: switch (support) {
              'automated' => Icons.check_circle,
              'guided' => Icons.menu_book,
              _ => Icons.help_outline,
            },
            text: switch (support) {
              'automated' => l.badgeAutomated(hostName),
              'guided' => l.badgeGuided(hostName),
              _ => l.badgeUntested(hostName),
            },
            color: support == 'automated' ? Colors.green : scheme.tertiary,
          );
    final status = switch (device.status) {
      'testing' => l.statusTesting,
      'candidate' => l.statusCandidate,
      _ => '',
    };
    return SizedBox(
      width: width,
      child: Card(
        key: ValueKey('device-${device.id}'),
        clipBehavior: Clip.antiAlias,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(
            color: selected ? scheme.primary : scheme.outlineVariant,
            width: selected ? 2 : 1,
          ),
        ),
        child: InkWell(
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      Icons.videogame_asset,
                      size: 40,
                      color: scheme.primary,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        status,
                        textAlign: TextAlign.end,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.labelSmall,
                      ),
                    ),
                    if (selected) ...[
                      const SizedBox(width: 8),
                      Icon(
                        Icons.check_circle,
                        key: ValueKey('selected-${device.id}'),
                        color: scheme.primary,
                      ),
                    ],
                  ],
                ),
                const SizedBox(height: 12),
                Text(
                  device.name,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                Text(
                  [
                    device.soc,
                    device.display,
                  ].where((x) => x.isNotEmpty).join(' · '),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 10),
                badge,
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _Badge extends StatelessWidget {
  final IconData icon;
  final String text;
  final Color color;
  const _Badge({
    super.key,
    required this.icon,
    required this.text,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 16, color: color),
          const SizedBox(width: 6),
          Flexible(
            child: Text(text, style: TextStyle(color: color)),
          ),
        ],
      ),
    );
  }
}
