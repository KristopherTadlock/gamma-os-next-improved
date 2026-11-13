import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';
import '../widgets/free_up_space.dart';

class DoneScreen extends StatelessWidget {
  const DoneScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final job = s.job;
    return StepPage(
      title: s.exportOnly ? l.imageReadyTitle : l.doneInstalledTitle,
      children: [
        if (s.exportOnly)
          Notice(
            key: const ValueKey('finish-on-windows'),
            icon: Icons.desktop_windows,
            title: l.finishOnWindows,
            body:
                '${needsWindowsText(l, s.device)}\n\n${l.finishOnWindowsBody}',
          )
        else
          Notice(
            icon: Icons.celebration,
            title: l.allDone,
            body: l.allDoneBody,
            color: Colors.green,
          ),
        const SizedBox(height: 16),
        if (job != null) SelectableText(job, textDirection: TextDirection.ltr),
        const SizedBox(height: 12),
        if (job != null)
          Align(
            alignment: AlignmentDirectional.centerStart,
            child: FilledButton.icon(
              onPressed: () => launchUrl(Uri.directory(job)),
              icon: const Icon(Icons.folder_open),
              label: Text(l.openJobFolder),
            ),
          ),
        const SizedBox(height: 24),
        const FreeUpSection(),
      ],
    );
  }
}
