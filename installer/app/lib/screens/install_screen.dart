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
    return StepPage(
      title: s.flashRunDry ? l.dryRun : l.installingTitle,
      subtitle: run.running && !s.flashRunDry ? l.keepConnected : null,
      children: [
        RunView(run, idleText: l.ready),
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
            if ((run.failed || (run.ok && s.flashRunDry)) && s.canRunAgain) ...[
              FilledButton(
                key: const ValueKey('run-again'),
                onPressed: s.runAgain,
                child: Text(l.runAgain),
              ),
              const SizedBox(width: 12),
            ],
            if (!run.running && !(run.ok && !s.flashRunDry))
              OutlinedButton(
                key: const ValueKey('back-to-review'),
                onPressed: () => s.goTo(WizardStep.review),
                child: Text(l.backToReview),
              ),
            if (run.running)
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
