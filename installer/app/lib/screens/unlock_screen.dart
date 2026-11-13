import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class UnlockScreen extends StatelessWidget {
  const UnlockScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final guides = s.plan?.guides;
    final route = s.unlockRoute;
    final explain = Notice(
      icon: Icons.lock_open,
      title: l.unlockExplainTitle,
      body: l.unlockExplainBody,
    );
    if (route == 'automated') {
      return StepPage(
        title: l.unlockTitle,
        children: [
          explain,
          const SizedBox(height: 12),
          GuideList(guides?.unlock ?? const []),
          const SizedBox(height: 12),
          FilledButton.icon(
            key: const ValueKey('run-unlock'),
            onPressed: s.unlockRun.running || s.unlockRun.ok
                ? null
                : s.startUnlock,
            icon: const Icon(Icons.lock_open),
            label: Text(l.unlockButton),
          ),
          const SizedBox(height: 12),
          RunView(s.unlockRun),
          if (s.unlockRun.running)
            Align(
              alignment: AlignmentDirectional.centerStart,
              child: OutlinedButton(
                key: const ValueKey('stop-unlock'),
                onPressed: s.cancelUnlock,
                child: Text(l.stop),
              ),
            ),
          if (s.unlockRun.ok)
            Notice(
              icon: Icons.check_circle,
              title: l.unlocked,
              color: Colors.green,
            ),
          CheckboxListTile(
            contentPadding: EdgeInsets.zero,
            value: s.unlockConfirmed,
            onChanged: (v) => s.setUnlockConfirmed(v ?? false),
            title: Text(l.alreadyUnlocked),
          ),
        ],
      );
    }
    if (route == 'guided') {
      return StepPage(
        title: l.unlockTitle,
        subtitle: l.unlockGuidedSubtitle,
        children: [
          explain,
          const SizedBox(height: 12),
          GuideList(guides?.unlock ?? const []),
          CheckboxListTile(
            key: const ValueKey('unlock-confirm'),
            contentPadding: EdgeInsets.zero,
            value: s.unlockConfirmed,
            onChanged: (v) => s.setUnlockConfirmed(v ?? false),
            title: Text(l.deviceUnlocked),
          ),
        ],
      );
    }
    return StepPage(title: l.unlockTitle, children: [Text(l.noUnlockNeeded)]);
  }
}
