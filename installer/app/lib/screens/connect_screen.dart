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
        if (s.detectError != null) ...[
          DetectErrorNotice(s),
          const SizedBox(height: 12),
        ],
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
        if (match == null && s.plan?.connectFirst == false)
          // Unisoc: the flash tool must already be waiting when the device
          // enters download mode, so it's connected after installing starts.
          Notice(
            key: const ValueKey('connect-later'),
            icon: Icons.info_outline,
            title: l.connectLaterTitle,
            body: l.connectLaterBody,
            color: scheme.primary,
          )
        else if (match != null)
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
