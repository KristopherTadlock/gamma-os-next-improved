import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../engine/engine.dart';
import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class DoneScreen extends StatelessWidget {
  const DoneScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final job = s.job;
    return StepPage(
      title: s.exportOnly ? l.imageReadyTitle : l.doneInstalledTitle,
      children: [
        if (s.exportOnly)
          Notice(
            key: const ValueKey('finish-on-windows'),
            icon: Icons.desktop_windows,
            title: l.finishOnWindows,
            body:
                '${needsWindowsText(l, s.device)}\n\n${l.finishOnWindowsBody}',
          )
        else
          Notice(
            icon: Icons.celebration,
            title: l.allDone,
            body: l.allDoneBody,
            color: Colors.green,
          ),
        const SizedBox(height: 16),
        if (job != null) SelectableText(job, textDirection: TextDirection.ltr),
        const SizedBox(height: 12),
        if (job != null)
          Align(
            alignment: AlignmentDirectional.centerStart,
            child: FilledButton.icon(
              onPressed: () => launchUrl(Uri.directory(job)),
              icon: const Icon(Icons.folder_open),
              label: Text(l.openJobFolder),
            ),
          ),
        const SizedBox(height: 24),
        const _FreeUpSpace(),
      ],
    );
  }
}

class _FreeUpSpace extends StatelessWidget {
  const _FreeUpSpace();

  String _kindLabel(AppLocalizations l, String kind) => switch (kind) {
    'downloads' => l.cleanDownloads,
    'image' => l.cleanImage,
    'backup' => l.cleanBackup,
    'scratch' => l.cleanScratch,
    _ => kind,
  };

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final items = s.cleanItems;
    if (items == null) return const SizedBox.shrink();
    final chosen = items
        .where((i) => s.cleanChoice[i.kind] ?? false)
        .fold<int>(0, (a, i) => a + i.bytes);
    return Column(
      key: const ValueKey('free-up-space'),
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(l.freeUpTitle, style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        if (items.isEmpty) Text(l.nothingToClean),
        for (final CleanItem i in items)
          CheckboxListTile(
            key: ValueKey('clean-${i.kind}'),
            contentPadding: EdgeInsets.zero,
            value: s.cleanChoice[i.kind] ?? false,
            onChanged: s.cleanBusy
                ? null
                : (v) => s.setCleanChoice(i.kind, v ?? false),
            title: Text('${_kindLabel(l, i.kind)} — ${formatBytes(i.bytes)}'),
            subtitle: i.kind == 'backup'
                ? Text(
                    l.cleanBackupWarning,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  )
                : Text(i.path, textDirection: TextDirection.ltr),
          ),
        if (items.isNotEmpty)
          Align(
            alignment: AlignmentDirectional.centerStart,
            child: OutlinedButton.icon(
              key: const ValueKey('run-clean'),
              onPressed: s.cleanBusy || chosen == 0 ? null : s.runClean,
              icon: const Icon(Icons.cleaning_services),
              label: Text(l.freeUpButton(formatBytes(chosen))),
            ),
          ),
        if (s.freedBytes != null)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text(
              l.freed(formatBytes(s.freedBytes!)),
              key: const ValueKey('freed'),
            ),
          ),
      ],
    );
  }
}
