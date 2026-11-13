import 'package:material_ui/material_ui.dart';
import 'package:path/path.dart' as p;
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
  late final AppState _state;

  @override
  void initState() {
    super.initState();
    final s = _state = context.read<AppState>();
    _workspace = TextEditingController(text: s.workspace);
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
    super.dispose();
  }

  Future<void> _changeFolder(AppState s) async {
    final dir = await pickDirectory();
    if (dir != null) {
      _workspace.text = dir;
      s.setWorkspace(dir);
    }
  }

  /// Opens the file picker; nothing chosen keeps the current source.
  Future<void> _chooseFiles(AppState s) async {
    final files = await pickPackFiles(AppLocalizations.of(context));
    s.setImportFiles(files);
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
              key: const ValueKey('change-folder'),
              onPressed: run.running ? null : () => _changeFolder(s),
              child: Text(l.changeFolder),
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
        Text(l.imageSourceTitle, style: Theme.of(context).textTheme.titleSmall),
        RadioGroup<ImageSource>(
          groupValue: s.imageSource,
          onChanged: (v) {
            if (run.running) return;
            if (v == ImageSource.download) {
              s.useDownload();
            } else if (s.importFiles.isNotEmpty) {
              s.setImportFiles(s.importFiles);
            } else {
              _chooseFiles(s);
            }
          },
          child: Column(
            children: [
              RadioListTile<ImageSource>(
                key: const ValueKey('source-download'),
                contentPadding: EdgeInsets.zero,
                value: ImageSource.download,
                enabled: !run.running,
                title: Text(l.imageSourceDownload),
              ),
              RadioListTile<ImageSource>(
                key: const ValueKey('source-files'),
                contentPadding: EdgeInsets.zero,
                value: ImageSource.files,
                enabled: !run.running,
                title: Text(l.imageSourceFiles),
                subtitle: Text(l.imageSourceFilesHelp),
              ),
            ],
          ),
        ),
        if (s.imageSource == ImageSource.files)
          Padding(
            key: const ValueKey('chosen-files'),
            padding: const EdgeInsetsDirectional.only(start: 48),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final f in s.importFiles)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 4),
                    child: Row(
                      children: [
                        const Icon(Icons.insert_drive_file_outlined, size: 18),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Tooltip(
                            message: f,
                            child: Text(
                              p.basename(f),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                TextButton(
                  key: const ValueKey('choose-files'),
                  onPressed: run.running ? null : () => _chooseFiles(s),
                  child: Text(l.chooseOtherFiles),
                ),
              ],
            ),
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
                  run.running ||
                      s.spaceChecking ||
                      s.workspace.trim().isEmpty ||
                      !s.imageSourceReady
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
