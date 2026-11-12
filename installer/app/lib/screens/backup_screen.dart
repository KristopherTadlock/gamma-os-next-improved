import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class BackupScreen extends StatelessWidget {
  const BackupScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final plan = s.plan;
    return StepPage(
      title: l.backupTitle,
      subtitle: (plan?.wipesData ?? true) ? l.backupSubtitle : null,
      children: [
        GuideList(plan?.guides.backup ?? const []),
        const SizedBox(height: 12),
        CheckboxListTile(
          key: const ValueKey('backed-up'),
          contentPadding: EdgeInsets.zero,
          value: s.backedUp,
          onChanged: (v) => s.setBackedUp(v ?? false),
          title: Text(l.backedUp),
        ),
      ],
    );
  }
}
