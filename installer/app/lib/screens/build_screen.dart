import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class BuildScreen extends StatefulWidget {
  const BuildScreen({super.key});

  @override
  State<BuildScreen> createState() => _BuildScreenState();
}

class _BuildScreenState extends State<BuildScreen> {
  late final TextEditingController _workspace;
  late final TextEditingController _import;
  late final AppState _state;

  @override
  void initState() {
    super.initState();
    final s = _state = context.read<AppState>();
    _workspace = TextEditingController(text: s.workspace);
    _import = TextEditingController(text: s.importDir);
    s.addListener(_syncWorkspace);
  }

  /// Keeps the field in step when the workspace changes elsewhere (the
  /// space dialog's "Choose another folder").
  void _syncWorkspace() {
    if (_workspace.text != _state.workspace) {
      _workspace.text = _state.workspace;
    }
  }

  @override
  void dispose() {
    _state.removeListener(_syncWorkspace);
    _workspace.dispose();
    _import.dispose();
    super.dispose();
  }

  Future<void> _pick(TextEditingController c, ValueChanged<String> set) async {
    final dir = await pickDirectory();
    if (dir != null) {
      c.text = dir;
      set(dir);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final run = s.buildRun;
    final space = s.space;
    final downloadsSize = space?.downloads ?? 0;
    return StepPage(
      title: l.buildTitle,
      subtitle: l.buildSubtitle,
      children: [
        if (s.exportOnly)
          Notice(
            icon: Icons.desktop_windows,
            title: l.exportOnlyTitle,
            body: l.exportOnlyBody,
          ),
        Row(
          children: [
            Expanded(
              child: TextField(
                key: const ValueKey('workspace-field'),
                controller: _workspace,
                enabled: !run.running,
                decoration: InputDecoration(
                  labelText: l.workspaceLabel,
                  helperText: l.workspaceHelp,
                ),
                onChanged: s.setWorkspace,
              ),
            ),
            const SizedBox(width: 8),
            OutlinedButton(
              onPressed: run.running
                  ? null
                  : () => _pick(_workspace, s.setWorkspace),
              child: Text(l.browse),
            ),
          ],
        ),
        const SizedBox(height: 8),
        if (s.spaceChecking)
          Text(l.spaceChecking, key: const ValueKey('space-checking'))
        else if (space != null)
          Text(
            l.spaceSummary(
              formatBytes(space.need),
              formatBytes(space.free),
              driveOf(s.workspace),
            ),
            key: const ValueKey('space-summary'),
            style: TextStyle(
              color: space.ok ? null : Theme.of(context).colorScheme.error,
            ),
          ),
        if (s.spaceError != null)
          Text(
            describeError(l, s.spaceError!),
            key: const ValueKey('space-error'),
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: TextField(
                key: const ValueKey('import-field'),
                controller: _import,
                enabled: !run.running,
                decoration: InputDecoration(
                  labelText: l.importLabel,
                  helperText: l.importHelp,
                ),
                onChanged: s.setImportDir,
              ),
            ),
            const SizedBox(width: 8),
            OutlinedButton(
              onPressed: run.running
                  ? null
                  : () => _pick(_import, s.setImportDir),
              child: Text(l.browse),
            ),
          ],
        ),
        const SizedBox(height: 8),
        CheckboxListTile(
          key: const ValueKey('delete-downloads'),
          contentPadding: EdgeInsets.zero,
          value: s.deleteDownloads,
          onChanged: run.running
              ? null
              : (v) => s.setDeleteDownloads(v ?? false),
          title: Text(l.deleteDownloads(formatBytes(downloadsSize))),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            FilledButton.icon(
              key: const ValueKey('start-build'),
              onPressed:
                  run.running || s.spaceChecking || s.workspace.trim().isEmpty
                  ? null
                  : s.startBuild,
              icon: const Icon(Icons.build),
              label: Text(run.result == null ? l.startBuild : l.buildAgain),
            ),
            const SizedBox(width: 12),
            if (run.running)
              OutlinedButton(onPressed: s.cancelBuild, child: Text(l.cancel)),
          ],
        ),
        const SizedBox(height: 20),
        RunView(run),
        if (run.ok)
          Notice(
            key: const ValueKey('build-complete'),
            icon: Icons.check_circle,
            title: l.buildComplete,
            body: s.image,
            color: Colors.green,
          ),
      ],
    );
  }
}
