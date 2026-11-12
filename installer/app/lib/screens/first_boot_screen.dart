import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class FirstBootScreen extends StatelessWidget {
  const FirstBootScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final steps = s.plan?.guides.firstBoot ?? const [];
    return StepPage(
      title: l.firstBootTitle,
      subtitle: l.firstBootSubtitle,
      children: [
        for (var i = 0; i < steps.length; i++)
          CheckboxListTile(
            contentPadding: EdgeInsets.zero,
            value: s.firstBootChecks.contains(i),
            onChanged: (_) => s.toggleFirstBoot(i),
            title: Text(steps[i].title),
            subtitle: Text(steps[i].body),
          ),
      ],
    );
  }
}
