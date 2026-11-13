import 'package:material_ui/material_ui.dart';
import 'package:file_selector/file_selector.dart';
import 'package:url_launcher/url_launcher.dart';

import '../engine/engine.dart';
import '../l10n/app_localizations.dart';
import '../state/app_state.dart';

String formatBytes(int bytes) {
  if (bytes <= 0) return '—';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  var v = bytes.toDouble();
  var u = 0;
  while (v >= 1000 && u < units.length - 1) {
    v /= 1000;
    u++;
  }
  return '${v.toStringAsFixed(v >= 100 || u == 0 ? 0 : 1)} ${units[u]}';
}

/// Localized label of a walkthrough step.
String stepLabel(AppLocalizations l, WizardStep s) => switch (s) {
  WizardStep.device => l.stepDevice,
  WizardStep.profile => l.stepProfile,
  WizardStep.journey => l.stepJourney,
  WizardStep.build => l.stepBuild,
  WizardStep.backup => l.stepBackup,
  WizardStep.unlock => l.stepUnlock,
  WizardStep.connect => l.stepConnect,
  WizardStep.review => l.stepReview,
  WizardStep.install => l.stepInstall,
  WizardStep.firstBoot => l.stepFirstBoot,
  WizardStep.done => l.stepDone,
};

/// The needs-Windows text for [d]: the catalog's own message, else ours.
String needsWindowsText(AppLocalizations l, DeviceInfo? d) =>
    d?.hostMessage ?? l.needsWindowsDefault;

/// Title, optional subtitle and a scrolling body for one step.
class StepPage extends StatelessWidget {
  final String title;
  final String? subtitle;
  final List<Widget> children;
  const StepPage({
    super.key,
    required this.title,
    this.subtitle,
    required this.children,
  });

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    return ListView(
      padding: const EdgeInsetsDirectional.fromSTEB(32, 28, 32, 24),
      children: [
        Text(title, style: t.headlineSmall),
        if (subtitle != null) ...[
          const SizedBox(height: 6),
          Text(subtitle!, style: t.bodyMedium),
        ],
        const SizedBox(height: 20),
        ...children,
      ],
    );
  }
}

/// A coloured information card (info, warning or error).
class Notice extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? body;
  final Color? color;
  final Widget? action;
  const Notice({
    super.key,
    required this.icon,
    required this.title,
    this.body,
    this.color,
    this.action,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final c = color ?? scheme.primary;
    return Card(
      color: c.withValues(alpha: 0.12),
      shape: RoundedRectangleBorder(
        side: BorderSide(color: c.withValues(alpha: 0.5)),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, color: c),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: Theme.of(context).textTheme.titleMedium),
                  if (body != null && body!.isNotEmpty) ...[
                    const SizedBox(height: 4),
                    Text(body!),
                  ],
                  if (action != null) ...[const SizedBox(height: 8), action!],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Numbered guide steps from the plan (texts come from the engine, already
/// in the chosen language).
class GuideList extends StatelessWidget {
  final List<GuideStep> steps;
  const GuideList(this.steps, {super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Column(
      children: [
        for (var i = 0; i < steps.length; i++)
          Card(
            child: ListTile(
              leading: CircleAvatar(child: Text('${i + 1}')),
              title: Text(steps[i].title),
              subtitle: Text(steps[i].body),
              trailing: steps[i].link == null
                  ? null
                  : IconButton(
                      tooltip: l.open,
                      icon: const Icon(Icons.open_in_new),
                      onPressed: () => launchUrl(Uri.parse(steps[i].link!)),
                    ),
            ),
          ),
      ],
    );
  }
}

/// Live view of a streaming engine command: stage, progress, prompts,
/// errors and the log.
class RunView extends StatelessWidget {
  final RunState run;
  final String idleText;
  const RunView(this.run, {super.key, this.idleText = ''});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final children = <Widget>[];
    if (run.stageTitle != null) {
      children.add(
        Text(run.stageTitle!, style: Theme.of(context).textTheme.titleMedium),
      );
      children.add(const SizedBox(height: 8));
    }
    if (run.running) {
      children.add(LinearProgressIndicator(value: run.fraction));
      if (run.label != null) {
        final pct = run.fraction == null
            ? ''
            : ' — ${(run.fraction! * 100).toStringAsFixed(0)}%';
        children.add(
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Text(
              '${run.label}$pct',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ),
        );
      }
    } else if (run.result == null && idleText.isNotEmpty) {
      children.add(Text(idleText));
    }
    if (run.prompt != null) {
      children.add(const SizedBox(height: 12));
      children.add(
        Notice(
          key: const ValueKey('prompt'),
          icon: Icons.back_hand,
          title: run.prompt!.title,
          body: run.prompt!.body,
          color: scheme.tertiary,
        ),
      );
    }
    final e = run.error;
    if (e != null &&
        e.code != 'unsupported_host' &&
        e.code != 'not_enough_space') {
      children.add(const SizedBox(height: 12));
      children.add(
        Notice(
          icon: Icons.error_outline,
          title: errorMessage(l, e),
          body: errorHint(l, e),
          color: scheme.error,
        ),
      );
    }
    if (run.log.isNotEmpty) {
      children.add(const SizedBox(height: 12));
      children.add(
        ExpansionTile(
          title: Text(l.details),
          tilePadding: EdgeInsets.zero,
          children: [
            Container(
              width: double.infinity,
              constraints: const BoxConstraints(maxHeight: 240),
              padding: const EdgeInsets.all(8),
              color: scheme.surfaceContainerHighest,
              child: SingleChildScrollView(
                reverse: true,
                child: SelectableText(
                  run.log.join('\n'),
                  textDirection: TextDirection.ltr,
                  style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                ),
              ),
            ),
          ],
        ),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: children,
    );
  }
}

/// The needs-Windows dialog: shown when this computer can't install the
/// device, from the device step or from an engine `unsupported_host` error.
Future<void> showNeedsWindowsDialog(
  BuildContext context, {
  required String message,
  required bool offerExport,
  required VoidCallback onExport,
}) {
  final l = AppLocalizations.of(context);
  return showDialog<void>(
    context: context,
    builder: (ctx) => AlertDialog(
      key: const ValueKey('needs-windows-dialog'),
      icon: const Icon(Icons.desktop_windows),
      title: Text(l.needsWindowsTitle),
      content: Text(message),
      actions: [
        if (offerExport)
          TextButton(
            key: const ValueKey('export-anyway'),
            onPressed: () {
              Navigator.of(ctx).pop();
              onExport();
            },
            child: Text(l.buildExportAnyway),
          ),
        FilledButton(
          key: const ValueKey('needs-windows-ok'),
          onPressed: () => Navigator.of(ctx).pop(),
          child: Text(l.ok),
        ),
      ],
    ),
  );
}

/// What the user chose in the not-enough-space dialog.
enum SpaceChoice { chooseFolder, checkAgain, cancel }

/// The not-enough-space dialog (build check or flash error).
Future<SpaceChoice> showSpaceDialog(
  BuildContext context, {
  required String message,
  required bool offerFolder,
}) async {
  final l = AppLocalizations.of(context);
  final r = await showDialog<SpaceChoice>(
    context: context,
    barrierDismissible: false,
    builder: (ctx) => AlertDialog(
      key: const ValueKey('space-dialog'),
      icon: const Icon(Icons.sd_storage),
      title: Text(l.spaceTitle),
      content: Text(message),
      actions: [
        TextButton(
          key: const ValueKey('space-cancel'),
          onPressed: () => Navigator.of(ctx).pop(SpaceChoice.cancel),
          child: Text(l.cancel),
        ),
        if (offerFolder)
          TextButton(
            key: const ValueKey('space-choose-folder'),
            onPressed: () => Navigator.of(ctx).pop(SpaceChoice.chooseFolder),
            child: Text(l.chooseAnotherFolder),
          ),
        FilledButton(
          key: const ValueKey('space-check-again'),
          onPressed: () => Navigator.of(ctx).pop(SpaceChoice.checkAgain),
          child: Text(l.checkAgain),
        ),
      ],
    ),
  );
  return r ?? SpaceChoice.cancel;
}

/// Opens the native folder picker. Replaceable in tests.
Future<String?> Function() pickDirectory = _pickWithFileSelector;

Future<String?> _pickWithFileSelector() async {
  try {
    return await getDirectoryPath();
  } catch (_) {
    // No native picker on this platform: the text field still works.
    return null;
  }
}

/// Opens the native picker for the Thimsy device pack: one `.tar` file or
/// several of its numbered parts (`.tar.001`, `.tar.002`, …). Returns the
/// chosen paths, empty when cancelled. Replaceable in tests.
Future<List<String>> Function(AppLocalizations l) pickPackFiles =
    _pickPackWithFileSelector;

Future<List<String>> _pickPackWithFileSelector(AppLocalizations l) async {
  try {
    final files = await openFiles(
      acceptedTypeGroups: [
        XTypeGroup(
          label: l.packFileType,
          extensions: [
            'tar',
            for (var i = 1; i < 100; i++) i.toString().padLeft(3, '0'),
          ],
        ),
        XTypeGroup(label: l.allFiles),
      ],
    );
    return [for (final f in files) f.path];
  } catch (_) {
    return const [];
  }
}

/// The message to show for [e]: the engine's own (already translated) text,
/// or the app's translation for errors it raised itself.
String errorMessage(AppLocalizations l, ErrorEvent e) {
  if (!e.local) return e.message;
  return switch (e.code) {
    'engine_crashed' => l.errEngineCrashed(e.args['exit'] ?? '?'),
    'engine_missing' => l.errEngineMissing,
    'cancelled' => l.errCancelled,
    'no_answer' => l.errNoAnswer,
    _ => e.message,
  };
}

String? errorHint(AppLocalizations l, ErrorEvent e) {
  if (!e.local) return e.hint;
  return switch (e.code) {
    'engine_crashed' => l.errEngineCrashedHint,
    'engine_missing' => l.errEngineMissingHint(e.args['path'] ?? ''),
    _ => null,
  };
}

/// Text for an error caught from a one-shot engine call.
String describeError(AppLocalizations l, Object error) {
  if (error is EngineException) {
    final hint = errorHint(l, error.event);
    final msg = errorMessage(l, error.event);
    return hint == null ? msg : '$msg\n$hint';
  }
  return '$error';
}
