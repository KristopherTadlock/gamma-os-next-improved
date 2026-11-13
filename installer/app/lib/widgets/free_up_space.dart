import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../engine/engine.dart';
import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import 'common.dart';

/// Kinds "Free up space" offers, in order. Backups are never among them.
const _kinds = ['scratch', 'downloads', 'image'];

String _kindLabel(AppLocalizations l, String kind, {required bool others}) =>
    switch (kind) {
      'downloads' => l.cleanDownloads,
      'image' => others ? l.cleanOtherImages : l.cleanImage,
      'backup' => l.cleanBackup,
      'scratch' => l.cleanScratch,
      _ => kind,
    };

Map<String, List<CleanItem>> _byKind(List<CleanItem> items) => {
  for (final k in _kinds)
    if (items.any((i) => i.kind == k)) k: [...items.where((i) => i.kind == k)],
};

int _bytes(Iterable<CleanItem> items) =>
    items.fold<int>(0, (a, i) => a + i.bytes);

/// The paths of a kind's items, a few at most, for a subtitle.
String _paths(List<CleanItem> items) {
  const shown = 3;
  final list = items.take(shown).map((i) => i.path).join('\n');
  return items.length > shown ? '$list\n…' : list;
}

/// One checkbox per kind, with the paths it would remove.
class _KindTile extends StatelessWidget {
  final String kind;
  final List<CleanItem> items;
  final bool value;
  final bool others;
  final ValueChanged<bool>? onChanged;
  final String keyPrefix;
  const _KindTile({
    required this.kind,
    required this.items,
    required this.value,
    required this.others,
    required this.onChanged,
    required this.keyPrefix,
  });

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return CheckboxListTile(
      key: ValueKey('$keyPrefix-$kind'),
      contentPadding: EdgeInsets.zero,
      value: value,
      onChanged: onChanged == null ? null : (v) => onChanged!(v ?? false),
      title: Text(
        '${_kindLabel(l, kind, others: others)} — '
        '${formatBytes(_bytes(items))}',
      ),
      subtitle: Text(_paths(items), textDirection: TextDirection.ltr),
    );
  }
}

/// "Free up space…" from the not-enough-space dialog or notice: lists what
/// `clean --dry-run` found (temporary files, downloads and other jobs'
/// images; never backups, which have their own confirmed "Delete
/// backups…"), removes the chosen items by path, and offers "Check again".
/// True when the user chose "Check again".
Future<bool> showFreeUpDialog(BuildContext context) async {
  final r = await showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (_) => const _FreeUpDialog(),
  );
  return r ?? false;
}

class _FreeUpDialog extends StatefulWidget {
  const _FreeUpDialog();

  @override
  State<_FreeUpDialog> createState() => _FreeUpDialogState();
}

class _FreeUpDialogState extends State<_FreeUpDialog> {
  /// Temporary files only by default: downloads may be the ones this build
  /// needs, and other images may still be wanted.
  final Set<String> _chosen = {'scratch'};
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    final s = context.read<AppState>();
    s.loadCleanItems().whenComplete(() {
      if (mounted) setState(() => _loading = false);
    });
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final groups = _byKind(s.freeableItems());
    final chosen = [
      for (final e in groups.entries)
        if (_chosen.contains(e.key)) ...e.value,
    ];
    final size = _bytes(chosen);
    return AlertDialog(
      key: const ValueKey('free-up-dialog'),
      icon: const Icon(Icons.cleaning_services),
      title: Text(l.freeUpTitle),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(l.freeUpSpaceHelp),
              const SizedBox(height: 8),
              if (_loading)
                const LinearProgressIndicator()
              else if (groups.isEmpty)
                Text(l.nothingToClean, key: const ValueKey('free-up-nothing')),
              for (final e in groups.entries)
                _KindTile(
                  kind: e.key,
                  items: e.value,
                  value: _chosen.contains(e.key),
                  others: true,
                  keyPrefix: 'free-up-kind',
                  onChanged: s.cleanBusy
                      ? null
                      : (v) => setState(
                          () => v ? _chosen.add(e.key) : _chosen.remove(e.key),
                        ),
                ),
              if (s.cleanBusy) const LinearProgressIndicator(),
              if (s.freedBytes != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    l.freed(formatBytes(s.freedBytes!)),
                    key: const ValueKey('free-up-freed'),
                  ),
                ),
              if (s.cleanError != null) ...[
                const SizedBox(height: 8),
                ErrorNotice(
                  key: const ValueKey('free-up-error'),
                  title: l.cleanFailed,
                  error: s.cleanError!,
                ),
              ],
              if (s.backupItems.isNotEmpty)
                Align(
                  alignment: AlignmentDirectional.centerStart,
                  child: TextButton(
                    key: const ValueKey('delete-backups'),
                    onPressed: s.cleanBusy
                        ? null
                        : () => showDeleteBackupsDialog(context),
                    child: Text(l.deleteBackupsButton),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const ValueKey('free-up-close'),
          onPressed: s.cleanBusy ? null : () => Navigator.of(context).pop(),
          child: Text(l.cancel),
        ),
        OutlinedButton(
          key: const ValueKey('free-up-run'),
          onPressed: s.cleanBusy || size == 0
              ? null
              : () => s.cleanPaths([for (final i in chosen) i.path]),
          child: Text(l.freeUpButton(formatBytes(size))),
        ),
        FilledButton(
          key: const ValueKey('free-up-check-again'),
          onPressed: s.cleanBusy ? null : () => Navigator.of(context).pop(true),
          child: Text(l.checkAgain),
        ),
      ],
    );
  }
}

/// Deleting backups: lists every backup run folder (device, time, path),
/// none chosen; only the folders the user ticks are deleted, after they
/// press "Delete selected".
Future<void> showDeleteBackupsDialog(BuildContext context) => showDialog<void>(
  context: context,
  barrierDismissible: false,
  builder: (_) => const _DeleteBackupsDialog(),
);

class _DeleteBackupsDialog extends StatefulWidget {
  const _DeleteBackupsDialog();

  @override
  State<_DeleteBackupsDialog> createState() => _DeleteBackupsDialogState();
}

class _DeleteBackupsDialogState extends State<_DeleteBackupsDialog> {
  final Set<String> _chosen = {};

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final scheme = Theme.of(context).colorScheme;
    final backups = s.backupItems;
    return AlertDialog(
      key: const ValueKey('delete-backups-dialog'),
      icon: Icon(Icons.warning_amber, color: scheme.error),
      title: Text(l.deleteBackupsTitle),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(l.deleteBackupsBody, style: TextStyle(color: scheme.error)),
              const SizedBox(height: 8),
              for (final (i, b) in backups.indexed)
                CheckboxListTile(
                  key: ValueKey('backup-$i'),
                  contentPadding: EdgeInsets.zero,
                  value: _chosen.contains(b.path),
                  onChanged: s.cleanBusy
                      ? null
                      : (v) => setState(
                          () => v == true
                              ? _chosen.add(b.path)
                              : _chosen.remove(b.path),
                        ),
                  title: Text(
                    '${b.backupDevice ?? '—'} · ${b.backupTime ?? '—'} · '
                    '${formatBytes(b.bytes)}',
                  ),
                  subtitle: Text(b.path, textDirection: TextDirection.ltr),
                ),
              if (s.cleanBusy) const LinearProgressIndicator(),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const ValueKey('delete-backups-cancel'),
          onPressed: s.cleanBusy ? null : () => Navigator.of(context).pop(),
          child: Text(l.cancel),
        ),
        FilledButton(
          key: const ValueKey('delete-backups-confirm'),
          style: FilledButton.styleFrom(
            backgroundColor: scheme.error,
            foregroundColor: scheme.onError,
          ),
          onPressed: s.cleanBusy || _chosen.isEmpty
              ? null
              : () async {
                  final nav = Navigator.of(context);
                  await s.deleteBackups(_chosen);
                  nav.pop();
                },
          child: Text(l.deleteSelected),
        ),
      ],
    );
  }
}

/// Done's "Free up space": per kind (temporary files and downloads chosen,
/// images not; backups only through "Delete backups…").
class FreeUpSection extends StatelessWidget {
  const FreeUpSection({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final items = s.cleanItems;
    if (items == null) return const SizedBox.shrink();
    final groups = _byKind(s.freeableItems(keepCurrentImage: false));
    final chosen = _bytes([
      for (final e in groups.entries)
        if (s.cleanChoice[e.key] ?? false) ...e.value,
    ]);
    return Column(
      key: const ValueKey('free-up-space'),
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(l.freeUpTitle, style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        if (groups.isEmpty && s.backupItems.isEmpty) Text(l.nothingToClean),
        for (final e in groups.entries)
          _KindTile(
            kind: e.key,
            items: e.value,
            value: s.cleanChoice[e.key] ?? false,
            others: false,
            keyPrefix: 'clean',
            onChanged: s.cleanBusy ? null : (v) => s.setCleanChoice(e.key, v),
          ),
        Wrap(
          spacing: 12,
          runSpacing: 8,
          children: [
            if (groups.isNotEmpty)
              OutlinedButton.icon(
                key: const ValueKey('run-clean'),
                onPressed: s.cleanBusy || chosen == 0 ? null : s.runClean,
                icon: const Icon(Icons.cleaning_services),
                label: Text(l.freeUpButton(formatBytes(chosen))),
              ),
            if (s.backupItems.isNotEmpty)
              TextButton(
                key: const ValueKey('delete-backups'),
                onPressed: s.cleanBusy
                    ? null
                    : () => showDeleteBackupsDialog(context),
                child: Text(l.deleteBackupsButton),
              ),
          ],
        ),
        if (s.backupItems.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(
              l.cleanBackupWarning,
              style: Theme.of(context).textTheme.bodySmall,
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
        if (s.cleanError != null) ...[
          const SizedBox(height: 8),
          ErrorNotice(
            key: const ValueKey('clean-error'),
            title: l.cleanFailed,
            error: s.cleanError!,
          ),
        ],
      ],
    );
  }
}
