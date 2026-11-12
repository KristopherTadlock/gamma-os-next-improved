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
      title: s.dryRun ? l.dryRun : l.installingTitle,
      subtitle: run.running && !s.dryRun ? l.keepConnected : null,
      children: [
        RunView(run, idleText: l.ready),
        if (run.ok)
          Notice(
            key: const ValueKey('install-complete'),
            icon: Icons.check_circle,
            title: s.dryRun ? l.dryRunComplete : l.installWritten,
            body: s.dryRun ? l.dryRunCompleteBody : l.installWrittenBody,
            color: Colors.green,
          ),
        const SizedBox(height: 12),
        Row(
          children: [
            if (run.failed || (run.ok && s.dryRun))
              FilledButton(
                key: const ValueKey('run-again'),
                onPressed: s.startFlash,
                child: Text(l.runAgain),
              ),
            if (run.running)
              OutlinedButton(onPressed: s.cancelFlash, child: Text(l.stop)),
          ],
        ),
        if (!run.running)
          ExpansionTile(
            tilePadding: EdgeInsets.zero,
            title: Text(l.advanced),
            children: [
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: Text(l.dryRun),
                subtitle: Text(l.dryRunHelp),
                value: s.dryRun,
                onChanged: s.setDryRun,
              ),
            ],
          ),
      ],
    );
  }
}
