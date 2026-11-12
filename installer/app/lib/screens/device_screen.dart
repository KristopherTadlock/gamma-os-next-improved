import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../engine/engine.dart';
import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

/// Host operating system names are product names: not translated.
const hostNames = {
  'windows': 'Windows',
  'macos': 'macOS',
  'linux': 'Linux',
  'android': 'Android',
};

class DeviceScreen extends StatelessWidget {
  const DeviceScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final hostName = hostNames[s.host] ?? s.host;
    if (s.catalogError != null) {
      return StepPage(
        title: l.deviceTitle,
        children: [
          Notice(
            icon: Icons.error_outline,
            title: l.deviceListError,
            body: describeError(l, s.catalogError!),
            color: Theme.of(context).colorScheme.error,
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
    return StepPage(
      title: l.deviceTitle,
      subtitle: l.runningOn(hostName),
      children: [
        SwitchListTile(
          key: const ValueKey('works-here'),
          contentPadding: EdgeInsets.zero,
          title: Text(l.onlyInstallable),
          value: s.worksHereOnly,
          onChanged: s.setWorksHereOnly,
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 16,
          runSpacing: 16,
          children: [
            for (final d in s.visibleDevices)
              _DeviceCard(
                device: d,
                host: s.host,
                hostName: hostName,
                selected: d.id == s.deviceId,
                onTap: () => s.selectDevice(d.id),
              ),
          ],
        ),
        if (s.device != null) ...[
          const SizedBox(height: 20),
          for (final n in s.device!.notes)
            Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Text('• $n'),
            ),
          for (final b in s.device!.blocks)
            Notice(
              icon: Icons.block,
              title: b,
              color: Theme.of(context).colorScheme.error,
            ),
        ],
      ],
    );
  }
}

class _DeviceCard extends StatelessWidget {
  final DeviceInfo device;
  final String host;
  final String hostName;
  final bool selected;
  final VoidCallback onTap;
  const _DeviceCard({
    required this.device,
    required this.host,
    required this.hostName,
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
      width: 260,
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
