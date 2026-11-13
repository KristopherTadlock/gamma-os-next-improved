import 'package:flutter/services.dart' show Clipboard, ClipboardData;
import 'package:material_ui/material_ui.dart';
import 'package:file_selector/file_selector.dart';
import 'package:provider/provider.dart';
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

  /// Shown under the body, before [action] (e.g. an expandable "Details").
  final Widget? footer;
  const Notice({
    super.key,
    required this.icon,
    required this.title,
    this.body,
    this.color,
    this.action,
    this.footer,
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
                  ?footer,
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

  /// Shown in the error notice, for the error the run ended with.
  final Widget? Function(ErrorEvent e)? errorAction;

  /// Added to the error notice's text, for the error the run ended with.
  final String? Function(ErrorEvent e)? errorExtra;
  const RunView(
    this.run, {
    super.key,
    this.idleText = '',
    this.errorAction,
    this.errorExtra,
  });

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
        ErrorNotice(
          key: const ValueKey('run-error'),
          error: e,
          run: run,
          extra: errorExtra?.call(e),
          action: errorAction?.call(e),
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

/// Asks before stopping a real install mid-way (the Stop button, or
/// closing the window). True when the user chose to stop.
Future<bool> confirmStopInstall(BuildContext context) async {
  final l = AppLocalizations.of(context);
  final stop = await showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (ctx) => AlertDialog(
      key: const ValueKey('confirm-stop'),
      icon: const Icon(Icons.warning_amber),
      title: Text(l.stopInstallTitle),
      content: Text(l.stopInstallBody),
      actions: [
        TextButton(
          key: const ValueKey('confirm-stop-yes'),
          onPressed: () => Navigator.of(ctx).pop(true),
          child: Text(l.stop),
        ),
        FilledButton(
          key: const ValueKey('confirm-stop-keep'),
          onPressed: () => Navigator.of(ctx).pop(false),
          child: Text(l.keepInstalling),
        ),
      ],
    ),
  );
  return stop ?? false;
}

/// What the user chose in the not-enough-space dialog.
enum SpaceChoice { chooseFolder, freeUp, checkAgain, cancel }

/// The not-enough-space dialog (build check or flash error). [details] is
/// the engine's own (English) message, under an expandable "Details".
Future<SpaceChoice> showSpaceDialog(
  BuildContext context, {
  required String message,
  required bool offerFolder,
  bool offerFreeUp = true,
  String? details,
}) async {
  final l = AppLocalizations.of(context);
  final r = await showDialog<SpaceChoice>(
    context: context,
    barrierDismissible: false,
    builder: (ctx) => AlertDialog(
      key: const ValueKey('space-dialog'),
      icon: const Icon(Icons.sd_storage),
      title: Text(l.spaceTitle),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(message),
            if (details != null && details.isNotEmpty && details != message)
              ErrorDetails(details),
          ],
        ),
      ),
      actions: [
        TextButton(
          key: const ValueKey('space-cancel'),
          onPressed: () => Navigator.of(ctx).pop(SpaceChoice.cancel),
          child: Text(l.cancel),
        ),
        if (offerFreeUp)
          TextButton(
            key: const ValueKey('space-free-up'),
            onPressed: () => Navigator.of(ctx).pop(SpaceChoice.freeUp),
            child: Text(l.freeUpSpaceButton),
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

/// The translated title of every error code the engine sends (see
/// installer/engine/crates/thimsy-engine/src/error.rs and
/// docs/engine-protocol.md); null for a code this app doesn't know.
String? errorTitleFor(AppLocalizations l, String code) => switch (code) {
  'device_not_found' => l.errTitleDeviceNotFound,
  'wrong_device' => l.errTitleWrongDevice,
  'download_failed' => l.errTitleDownloadFailed,
  'hash_mismatch' => l.errTitleHashMismatch,
  'not_enough_space' => l.spaceTitle,
  'tool_missing' => l.errTitleToolMissing,
  'tool_failed' => l.errTitleToolFailed,
  'unsupported_host' => l.needsWindowsTitle,
  'profile_unavailable' => l.errTitleProfileUnavailable,
  'unlock_guided' => l.errTitleUnlockGuided,
  'cancelled' => l.errCancelled,
  'bad_input' => l.errTitleBadInput,
  'internal' => l.errTitleInternal,
  'busy' => l.errTitleBusy,
  'full_install_needed' => l.errTitleFullInstallNeeded,
  _ => null,
};

/// The heading for [e]: the app's translation for its code (the engine's
/// `message` stays English, so it's never the heading). Errors the app
/// raised itself have their own translated text.
String errorTitle(AppLocalizations l, ErrorEvent e) {
  if (e.local) return errorMessage(l, e);
  return errorTitleFor(l, e.code) ?? l.errTitleUnknown;
}

/// The app's translation for an error it raised itself; for engine errors,
/// [errorTitle].
String errorMessage(AppLocalizations l, ErrorEvent e) {
  if (!e.local) return errorTitle(l, e);
  return switch (e.code) {
    'engine_crashed' => l.errEngineCrashed(e.args['exit'] ?? '?'),
    'engine_missing' => l.errEngineMissing,
    'cancelled' => l.errCancelled,
    'no_answer' => l.errNoAnswer,
    'detect_stopped' => l.detectStopped,
    _ => e.message,
  };
}

/// What to do about [e]: the engine's hint (translated by the engine), or
/// the app's own for errors it raised.
String? errorHint(AppLocalizations l, ErrorEvent e) {
  if (!e.local) return e.hint;
  return switch (e.code) {
    'engine_crashed' => l.errEngineCrashedHint,
    'engine_missing' => l.errEngineMissingHint(e.args['path'] ?? ''),
    _ => null,
  };
}

/// The engine's (English) message for [e], for "Details"; null when it adds
/// nothing.
String? errorDetails(ErrorEvent e) =>
    e.message.trim().isEmpty ? null : e.message;

/// [error] (from a one-shot engine call, or anything else thrown) as an
/// error event.
ErrorEvent errorEventOf(Object error) => error is EngineException
    ? error.event
    : ErrorEvent(code: 'internal', message: '$error');

/// Text for an error caught from a one-shot engine call: its title and
/// hint (the English message is left to "Details").
String describeError(AppLocalizations l, Object error) {
  final e = errorEventOf(error);
  final hint = errorHint(l, e);
  final title = errorTitle(l, e);
  return hint == null || hint.isEmpty ? title : '$title\n$hint';
}

/// An expandable "Details" holding the engine's English message.
class ErrorDetails extends StatelessWidget {
  final String text;
  const ErrorDetails(this.text, {super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return ExpansionTile(
      key: const ValueKey('error-details'),
      title: Text(l.details),
      tilePadding: EdgeInsets.zero,
      childrenPadding: const EdgeInsets.only(bottom: 8),
      expandedAlignment: AlignmentDirectional.centerStart,
      children: [
        SelectableText(
          text,
          key: const ValueKey('error-details-text'),
          textDirection: TextDirection.ltr,
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

/// An error notice: the translated title, the hint as the body, the
/// engine's English message under "Details", and "Copy details" (the
/// engine's stderr log plus [run]'s log, for a report).
class ErrorNotice extends StatelessWidget {
  final ErrorEvent error;
  final RunState? run;

  /// A heading for where it happened (e.g. "The device list could not be
  /// loaded"); the error's own title then goes in the body.
  final String? title;

  /// More text after the hint.
  final String? extra;
  final IconData icon;
  final Widget? action;
  const ErrorNotice({
    super.key,
    required this.error,
    this.run,
    this.title,
    this.extra,
    this.icon = Icons.error_outline,
    this.action,
  });

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final own = errorTitle(l, error);
    final heading = title ?? own;
    final hint = errorHint(l, error);
    final details = errorDetails(error);
    return Notice(
      icon: icon,
      title: heading,
      body: [
        if (own != heading) own,
        ?hint,
        ?extra,
      ].where((t) => t.trim().isNotEmpty).join('\n'),
      color: Theme.of(context).colorScheme.error,
      footer: details == null || details == own || error.local
          ? null
          : ErrorDetails(details),
      action: Wrap(
        spacing: 8,
        runSpacing: 8,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          ?action,
          CopyDetailsButton(run: run),
        ],
      ),
    );
  }
}

/// Copies the engine's stderr log and [run]'s log to the clipboard.
class CopyDetailsButton extends StatelessWidget {
  final RunState? run;
  const CopyDetailsButton({super.key, this.run});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return TextButton.icon(
      key: const ValueKey('copy-details'),
      icon: const Icon(Icons.copy, size: 18),
      label: Text(l.copyDetails),
      onPressed: () async {
        final text = context.read<AppState>().diagnostics(run);
        final messenger = ScaffoldMessenger.maybeOf(context);
        await Clipboard.setData(ClipboardData(text: text));
        messenger?.showSnackBar(SnackBar(content: Text(l.detailsCopied)));
      },
    );
  }
}

/// Shown on Connect and Review when the device watch ended by itself: no
/// device list is shown from it any more, and "Try again" restarts it.
class DetectErrorNotice extends StatelessWidget {
  final AppState state;
  const DetectErrorNotice(this.state, {super.key});

  @override
  Widget build(BuildContext context) {
    final e = state.detectError;
    if (e == null) return const SizedBox.shrink();
    final l = AppLocalizations.of(context);
    return ErrorNotice(
      key: const ValueKey('detect-error'),
      icon: Icons.usb_off,
      title: l.detectStopped,
      error: e,
      action: TextButton(
        key: const ValueKey('detect-retry'),
        onPressed: state.restartDetect,
        child: Text(l.tryAgain),
      ),
    );
  }
}

/// The space check's answer as a sentence, for a check that said no.
String spaceProblemText(
  AppLocalizations l, {
  required String? reason,
  required int need,
  required int free,
  required String drive,
}) => switch (reason) {
  'fat32' => l.spaceFat32,
  'probe_failed' => l.spaceProbeFailed,
  _ => l.spaceBody(formatBytes(need), formatBytes(free), drive),
};

/// macOS on Apple silicon asks before a new USB accessory may connect, and
/// a Unisoc device leaves download mode within seconds if it isn't answered.
class MacAccessoryNotice extends StatelessWidget {
  final String device;
  const MacAccessoryNotice(this.device, {super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Notice(
      key: const ValueKey('mac-accessory'),
      icon: Icons.laptop_mac,
      title: l.macAccessoryTitle,
      body: l.macAccessoryBody(device),
      color: Theme.of(context).colorScheme.tertiary,
    );
  }
}
