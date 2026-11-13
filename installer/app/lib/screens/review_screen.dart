import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../engine/engine.dart';
import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class ReviewScreen extends StatelessWidget {
  const ReviewScreen({super.key});

  Future<void> _confirm(BuildContext context, AppState s) async {
    final l = AppLocalizations.of(context);
    final full = s.flashMode == FlashMode.full;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        key: const ValueKey('confirm-install'),
        title: Text(s.dryRun ? l.confirmDryRunTitle : l.confirmInstallTitle),
        content: Text(
          s.dryRun
              ? l.confirmDryRunBody
              : full
              ? l.confirmFullBody(s.device?.name ?? '')
              : l.confirmUpdateBody,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(l.cancel),
          ),
          FilledButton(
            key: const ValueKey('confirm-install-yes'),
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(s.dryRun ? l.startDryRun : l.installButton),
          ),
        ],
      ),
    );
    if (ok == true) s.install();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final scheme = Theme.of(context).colorScheme;
    final plan = s.plan;
    Widget row(String k, String v) => Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(width: 160, child: Text(k)),
          Expanded(child: SelectableText(v)),
        ],
      ),
    );
    final profileName =
        s.catalog?.profiles
            .where((p) => p.id == s.profileId)
            .map((p) => p.name)
            .firstOrNull ??
        '—';
    return StepPage(
      title: l.reviewTitle,
      children: [
        row(l.rowDevice, s.device?.name ?? '—'),
        row(
          l.rowConnectedAs,
          s.matchedDevice?.label ?? (s.dryRun ? l.dryRunValue : '—'),
        ),
        row(l.rowProfile, profileName),
        row(l.rowImage, s.image ?? '—'),
        row(l.rowPartitions, plan?.partitions.join(', ') ?? '—'),
        if (s.detectError != null) ...[
          const SizedBox(height: 12),
          DetectErrorNotice(s),
        ],
        const SizedBox(height: 16),
        Text(l.howToInstall, style: Theme.of(context).textTheme.titleMedium),
        RadioGroup<FlashMode>(
          groupValue: s.flashMode,
          onChanged: (m) => s.setFlashMode(m ?? s.allowedModes.first),
          child: Column(
            children: [
              for (final m in s.allowedModes)
                if (m == FlashMode.update)
                  RadioListTile<FlashMode>(
                    key: const ValueKey('mode-update'),
                    contentPadding: EdgeInsets.zero,
                    value: FlashMode.update,
                    title: Text(l.modeUpdate),
                    subtitle: Text(
                      (s.journey?.updateNote?.isNotEmpty ?? false)
                          ? s.journey!.updateNote!
                          : l.journeyUpdateNote,
                    ),
                  )
                else
                  RadioListTile<FlashMode>(
                    key: const ValueKey('mode-full'),
                    contentPadding: EdgeInsets.zero,
                    value: FlashMode.full,
                    title: Text(
                      s.allowedModes.length > 1 ? l.modeReinstall : l.modeFresh,
                    ),
                  ),
            ],
          ),
        ),
        if (s.flashMode == FlashMode.full && (plan?.wipesData ?? true))
          Notice(
            icon: Icons.warning_amber,
            title: l.eraseWarning,
            color: scheme.error,
          ),
        ExpansionTile(
          tilePadding: EdgeInsets.zero,
          title: Text(l.advanced),
          children: [
            SwitchListTile(
              key: const ValueKey('dry-run'),
              contentPadding: EdgeInsets.zero,
              title: Text(l.dryRun),
              subtitle: Text(l.dryRunHelp),
              value: s.dryRun,
              onChanged: s.setDryRun,
            ),
          ],
        ),
        const SizedBox(height: 16),
        Align(
          alignment: AlignmentDirectional.centerStart,
          child: FilledButton.icon(
            key: const ValueKey('install-button'),
            style: FilledButton.styleFrom(
              padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 20),
            ),
            onPressed: s.job == null || (!s.dryRun && !s.canInstallForReal)
                ? null
                : () => _confirm(context, s),
            icon: const Icon(Icons.system_update),
            label: Text(s.dryRun ? l.startDryRun : l.installButton),
          ),
        ),
      ],
    );
  }
}
