import 'dart:io' show Platform;
import 'dart:ui' show AppExitResponse;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../l10n/languages.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';
import 'backup_screen.dart';
import 'build_screen.dart';
import 'connect_screen.dart';
import 'device_screen.dart';
import 'done_screen.dart';
import 'first_boot_screen.dart';
import 'install_screen.dart';
import 'journey_screen.dart';
import 'profile_screen.dart';
import 'review_screen.dart';
import 'unlock_screen.dart';

/// The single installer window: step rail at the start side, the current
/// step beside it, Back / Next at the bottom. Mirrors for RTL languages.
class HomeShell extends StatefulWidget {
  /// Whether closing the window during a real install asks first. Defaults
  /// to Windows and Linux, where the close button goes through the app; on
  /// macOS the window is gone before the app is asked, so there's nothing
  /// to show the question in.
  final bool? guardWindowClose;

  const HomeShell({super.key, this.guardWindowClose});

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  bool _dialogOpen = false;
  late final AppLifecycleListener _lifecycle;

  @override
  void initState() {
    super.initState();
    final s = context.read<AppState>();
    if (s.catalog == null) {
      WidgetsBinding.instance.addPostFrameCallback((_) => s.loadCatalog());
    }
    _lifecycle = AppLifecycleListener(onExitRequested: _onExitRequested);
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  bool get _guardClose =>
      widget.guardWindowClose ??
      (!kIsWeb && (Platform.isWindows || Platform.isLinux));

  /// Closing the window mid-install: same question as Stop.
  Future<AppExitResponse> _onExitRequested() async {
    final s = context.read<AppState>();
    if (!_guardClose || !s.stopNeedsConfirmation) return AppExitResponse.exit;
    if (!await confirmStopInstall(context)) return AppExitResponse.cancel;
    await s.cancelFlash();
    return AppExitResponse.exit;
  }

  Widget _screen(WizardStep step) => switch (step) {
    WizardStep.device => const DeviceScreen(),
    WizardStep.profile => const ProfileScreen(),
    WizardStep.journey => const JourneyScreen(),
    WizardStep.build => const BuildScreen(),
    WizardStep.backup => const BackupScreen(),
    WizardStep.unlock => const UnlockScreen(),
    WizardStep.connect => const ConnectScreen(),
    WizardStep.review => const ReviewScreen(),
    WizardStep.install => const InstallScreen(),
    WizardStep.firstBoot => const FirstBootScreen(),
    WizardStep.done => const DoneScreen(),
  };

  Future<void> _needsWindows(AppState s, String message) async {
    if (_dialogOpen) return;
    _dialogOpen = true;
    await showNeedsWindowsDialog(
      context,
      message: message,
      offerExport: s.device?.buildVerified ?? false,
      onExport: s.step == WizardStep.device
          ? s.continueExportOnly
          : s.exportAnyway,
    );
    _dialogOpen = false;
  }

  Future<void> _space(AppState s, SpaceProblem p) async {
    if (_dialogOpen) return;
    _dialogOpen = true;
    final l = AppLocalizations.of(context);
    final message = p.kind == SpaceProblemKind.flash || p.need == 0
        ? l.spaceFlashBody(p.message ?? l.spaceTitle)
        : l.spaceBody(
            formatBytes(p.need),
            formatBytes(p.free),
            driveOf(s.workspace),
          );
    final choice = await showSpaceDialog(
      context,
      message: message,
      offerFolder: p.kind == SpaceProblemKind.build,
    );
    _dialogOpen = false;
    switch (choice) {
      case SpaceChoice.checkAgain:
        await s.recheckSpace();
      case SpaceChoice.chooseFolder:
        final dir = await pickDirectory();
        s.clearSpaceProblem();
        if (dir != null) s.setWorkspace(dir);
      case SpaceChoice.cancel:
        s.clearSpaceProblem();
    }
  }

  void _next(AppState s) {
    if (s.step == WizardStep.device) {
      if (s.deviceNext() == DeviceNext.needsWindows) {
        _needsWindows(
          s,
          needsWindowsText(AppLocalizations.of(context), s.device),
        );
      }
      return;
    }
    s.next();
  }

  Future<void> _pickLanguage(AppState s) async {
    final l = AppLocalizations.of(context);
    final picked = await showDialog<String>(
      context: context,
      builder: (ctx) => SimpleDialog(
        key: const ValueKey('language-dialog'),
        title: Text(l.language),
        children: [
          RadioGroup<String>(
            groupValue: s.languageCode ?? 'system',
            onChanged: (v) => Navigator.of(ctx).pop(v),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                RadioListTile<String>(
                  key: const ValueKey('lang-system'),
                  value: 'system',
                  title: Text(l.languageSystem),
                ),
                for (final lang in languages)
                  RadioListTile<String>(
                    key: ValueKey('lang-${lang.code}'),
                    value: lang.code,
                    title: Text(lang.nativeName),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
    if (picked == null) return;
    s.setLanguage(picked == 'system' ? null : picked);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final err = s.hostError;
    if (err != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        s.clearHostError();
        _needsWindows(s, err.message);
      });
    }
    final space = s.spaceProblem;
    if (space != null && !_dialogOpen) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted || s.spaceProblem != space) return;
        _space(s, space);
      });
    }
    final scheme = Theme.of(context).colorScheme;
    final showNext = s.step != WizardStep.review && s.step != WizardStep.done;
    return Scaffold(
      body: Row(
        children: [
          Material(
            color: scheme.surfaceContainerLow,
            child: SizedBox(
              width: 240,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Padding(
                    padding: const EdgeInsetsDirectional.fromSTEB(
                      20,
                      20,
                      8,
                      12,
                    ),
                    child: Row(
                      children: [
                        Expanded(
                          child: Text(
                            l.appTitle,
                            style: Theme.of(context).textTheme.titleLarge,
                          ),
                        ),
                        IconButton(
                          key: const ValueKey('language-button'),
                          tooltip: l.language,
                          icon: const Icon(Icons.language),
                          onPressed: () => _pickLanguage(s),
                        ),
                      ],
                    ),
                  ),
                  Expanded(
                    child: ListView(
                      children: [
                        for (final step in WizardStep.values)
                          if (!s.isSkipped(step))
                            ListTile(
                              key: ValueKey('rail-${step.name}'),
                              dense: true,
                              selected: step == s.step,
                              // Locked while a build or install runs.
                              enabled: s.canGoTo(step),
                              leading: Icon(
                                step.index < s.furthest.index
                                    ? Icons.check_circle
                                    : step == s.step
                                    ? Icons.radio_button_checked
                                    : Icons.radio_button_unchecked,
                                size: 20,
                              ),
                              title: Text(stepLabel(l, step)),
                              onTap: () => s.goTo(step),
                            ),
                      ],
                    ),
                  ),
                  if (s.catalog != null)
                    Padding(
                      padding: const EdgeInsets.all(16),
                      child: Text(
                        l.catalogVersion(s.catalog!.version),
                        style: Theme.of(context).textTheme.labelSmall,
                      ),
                    ),
                ],
              ),
            ),
          ),
          Expanded(
            child: Column(
              children: [
                Expanded(child: _screen(s.step)),
                const Divider(height: 1),
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 32,
                    vertical: 12,
                  ),
                  child: Row(
                    children: [
                      if (s.step != WizardStep.device &&
                          s.step != WizardStep.install)
                        TextButton(
                          key: const ValueKey('back'),
                          onPressed: s.busy ? null : s.back,
                          child: Text(l.back),
                        ),
                      const Spacer(),
                      if (showNext)
                        FilledButton(
                          key: const ValueKey('next'),
                          onPressed: s.canProceed(s.step)
                              ? () => _next(s)
                              : null,
                          child: Text(l.next),
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Shown on Android until the engine sidecar ships there.
class AndroidNotice extends StatelessWidget {
  const AndroidNotice({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Scaffold(
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Notice(
            key: const ValueKey('android-notice'),
            icon: Icons.phone_android,
            title: l.androidTitle,
            body: l.androidBody,
          ),
        ),
      ),
    );
  }
}
