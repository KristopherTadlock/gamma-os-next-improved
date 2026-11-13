import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class InstallScreen extends StatelessWidget {
  const InstallScreen({super.key});

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
            if (run.failed || (run.ok && s.flashRunDry))
              FilledButton(
                key: const ValueKey('run-again'),
                onPressed: s.runAgain,
                child: Text(l.runAgain),
              ),
            if (run.ok && s.flashRunDry) ...[
              const SizedBox(width: 12),
              OutlinedButton(
                key: const ValueKey('back-to-review'),
                onPressed: () => s.goTo(WizardStep.review),
                child: Text(l.backToReview),
              ),
            ],
            if (run.running)
              OutlinedButton(onPressed: s.cancelFlash, child: Text(l.stop)),
          ],
        ),
      ],
    );
  }
}
