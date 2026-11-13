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
    // A failed install's English message, under "Details".
    final driverError = s.driverError;
    final driverDetails = driverError == null
        ? null
        : errorDetails(driverError);
    // Unisoc: the device is connected only once installing has started (the
    // flash tool must already be waiting), so this step only sets up USB
    // and shows what comes then.
    final later = s.plan?.connectFirst == false;
    return StepPage(
      title: later ? l.connectSetupTitle : l.connectTitle,
      subtitle: later
          ? l.connectSetupSubtitle(s.device?.name ?? '')
          : l.connectSubtitle,
      children: [
        if (later) ...[
          Text(
            l.connectStepsLater,
            key: const ValueKey('connect-steps-later'),
            style: Theme.of(context).textTheme.titleSmall,
          ),
          const SizedBox(height: 8),
        ],
        GuideList(s.plan?.guides.downloadMode ?? const []),
        const SizedBox(height: 12),
        if (s.detectError != null) ...[
          DetectErrorNotice(s),
          const SizedBox(height: 12),
        ],
        // Windows: the WinUSB driver; Linux: a udev rule for USB access.
        // macOS needs neither and never shows this.
        if (s.checksDriver && s.driverStatus == 'missing')
          Notice(
            key: const ValueKey('driver-notice'),
            icon: Icons.usb,
            title: s.host == 'linux' ? l.driverNeededLinux : l.driverNeeded,
            body: [
              if (s.host == 'linux') l.driverLinuxBody,
              ?s.driverDetail,
            ].where((t) => t.isNotEmpty).join('\n'),
            color: s.driverError == null ? scheme.tertiary : scheme.error,
            footer: driverDetails == null || driverDetails == s.driverDetail
                ? null
                : ErrorDetails(driverDetails),
            action: FilledButton(
              key: const ValueKey('install-driver'),
              onPressed: s.driverBusy
                  ? null
                  : () => s.checkDriver(install: true),
              child: Text(
                s.host == 'linux' ? l.installButton : l.installDriver,
              ),
            ),
          ),
        if (match == null && later)
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
        if (match == null && !later)
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
