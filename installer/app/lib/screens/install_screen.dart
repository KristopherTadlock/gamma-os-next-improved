import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class InstallScreen extends StatelessWidget {
  const InstallScreen({super.key});

  /// Stop: once a real install is writing the device, asks first.
  Future<void> _stop(BuildContext context, AppState s) async {
    if (s.stopNeedsConfirmation && !await confirmStopInstall(context)) return;
    await s.cancelFlash();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final run = s.flashRun;
    final code = run.failed ? run.error?.code : null;
    // The engine refused an update: repeating it would fail the same way.
    final needsFull = code == 'full_install_needed';
    // Waiting for the background preparation: its progress is shown, and
    // the install starts when it ends.
    final pending = s.installPending;
    final shown = pending ? s.prepareRun : run;
    final name = s.device?.name ?? '';
    final stage = run.stage;
    // What the page says follows what's happening: getting ready (nothing
    // touches the device), waiting for the device (nothing written yet),
    // then installing.
    final String title;
    String? subtitle;
    if (s.flashRunDry) {
      title = l.dryRun;
    } else if (pending ||
        (run.running &&
            (stage == null ||
                stage == 'check' ||
                stage == 'driver' ||
                stage == 'prepare'))) {
      title = l.gettingReadyTitle;
      subtitle = l.gettingReadyBody;
    } else if (run.running && stage == 'wait_device') {
      title = l.connectNowTitle(name);
      subtitle = l.waitingForDevice(name);
    } else {
      title = l.installingTitle;
      subtitle = run.running ? l.keepConnected : null;
    }
    return StepPage(
      title: title,
      subtitle: subtitle,
      children: [
        RunView(
          shown,
          idleText: l.ready,
          errorExtra: (e) =>
              e.code == 'full_install_needed' ? l.fullInstallNeededBody : null,
          errorAction: (e) => e.code == 'full_install_needed'
              ? FilledButton(
                  key: const ValueKey('switch-to-full'),
                  onPressed: s.busy ? null : s.switchToFullInstall,
                  child: Text(l.switchToFull),
                )
              : null,
        ),
        // The device never entered download mode: the steps again, then
        // Run again (the same confirmed install).
        if (code == 'device_not_found') ...[
          const SizedBox(height: 12),
          Text(
            l.downloadModeAgain,
            key: const ValueKey('download-mode-again'),
            style: Theme.of(context).textTheme.titleSmall,
          ),
          const SizedBox(height: 8),
          GuideList(s.plan?.guides.downloadMode ?? const []),
        ],
        if (run.ok)
          Notice(
            key: const ValueKey('install-complete'),
            icon: Icons.check_circle,
            title: s.flashRunDry ? l.dryRunComplete : l.installWritten,
            body: s.flashRunDry ? l.dryRunCompleteBody : l.installWrittenBody,
            color: Colors.green,
          ),
        const SizedBox(height: 12),
        Row(
          children: [
            // "Run again" repeats only what was confirmed on Review.
            if ((run.failed || (run.ok && s.flashRunDry)) &&
                s.canRunAgain &&
                !needsFull) ...[
              FilledButton(
                key: const ValueKey('run-again'),
                onPressed: s.runAgain,
                child: Text(l.runAgain),
              ),
              const SizedBox(width: 12),
            ],
            if (!run.running && !pending && !(run.ok && !s.flashRunDry))
              OutlinedButton(
                key: const ValueKey('back-to-review'),
                onPressed: () => s.goTo(WizardStep.review),
                child: Text(l.backToReview),
              ),
            if (run.running || pending)
              OutlinedButton(
                key: const ValueKey('stop-install'),
                onPressed: () => _stop(context, s),
                child: Text(l.stop),
              ),
          ],
        ),
      ],
    );
  }
}
