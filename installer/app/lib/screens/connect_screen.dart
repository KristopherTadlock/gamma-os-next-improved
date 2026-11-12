import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class ConnectScreen extends StatelessWidget {
  const ConnectScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final scheme = Theme.of(context).colorScheme;
    final match = s.matchedDevice;
    return StepPage(
      title: l.connectTitle,
      subtitle: l.connectSubtitle,
      children: [
        GuideList(s.plan?.guides.downloadMode ?? const []),
        const SizedBox(height: 12),
        if (s.host == 'windows' && s.driverStatus == 'missing')
          Notice(
            icon: Icons.usb,
            title: l.driverNeeded,
            body: s.driverDetail,
            color: scheme.tertiary,
            action: FilledButton(
              key: const ValueKey('install-driver'),
              onPressed: s.driverBusy
                  ? null
                  : () => s.checkDriver(install: true),
              child: Text(l.installDriver),
            ),
          ),
        if (match != null)
          Notice(
            key: const ValueKey('device-connected'),
            icon: Icons.check_circle,
            title: l.connected(match.label),
            body: '${match.vidpid} · ${match.mode}',
            color: Colors.green,
          )
        else
          Notice(
            icon: Icons.usb,
            title: l.waitingDevice,
            body: s.detected.isEmpty
                ? l.nothingConnected
                : l.seenOther(
                    s.detected.map((d) => '${d.label} (${d.mode})').join(', '),
                  ),
          ),
        const SizedBox(height: 12),
        if (match == null)
          Align(
            alignment: AlignmentDirectional.centerStart,
            child: TextButton(
              key: const ValueKey('dry-run-continue'),
              onPressed: () {
                s.setDryRun(true);
                s.next();
              },
              child: Text(l.continueDryRun),
            ),
          ),
      ],
    );
  }
}
