import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'engine/engine.dart';
import 'engine/fake_engine.dart';
import 'engine/process_engine.dart';
import 'l10n/app_localizations.dart';
import 'l10n/languages.dart';
import 'screens/home_shell.dart';
import 'state/app_state.dart';
import 'state/default_workspace.dart';

const _useFake = bool.fromEnvironment('FAKE_ENGINE');

/// Remembers the chosen language with shared_preferences.
class PrefsLanguageStore implements LanguageStore {
  static const _key = 'language';
  final SharedPreferences prefs;
  PrefsLanguageStore(this.prefs);

  @override
  String? load() => prefs.getString(_key);

  @override
  Future<void> save(String? code) =>
      code == null ? prefs.remove(_key) : prefs.setString(_key, code);
}

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  LanguageStore? store;
  try {
    store = PrefsLanguageStore(await SharedPreferences.getInstance());
  } catch (_) {
    store = null; // no persistence available; follow the system language
  }
  final Engine engine = _useFake ? FakeEngine() : ProcessEngine();
  runApp(
    ThimsyInstallerApp(
      state: AppState(
        engine,
        languageStore: store,
        // Next to the app (or the app data folder when that's read-only),
        // so nobody has to pick a folder; the build step can change it.
        workspace: findDefaultWorkspace(),
      ),
    ),
  );
}

class ThimsyInstallerApp extends StatelessWidget {
  final AppState state;

  /// Whether closing the window mid-install asks first; null for the
  /// platform default ([HomeShell.defaultGuardWindowClose]).
  final bool? guardWindowClose;

  const ThimsyInstallerApp({
    super.key,
    required this.state,
    this.guardWindowClose,
  });

  @override
  Widget build(BuildContext context) {
    const seed = Color(0xFF7E57C2);
    return ChangeNotifierProvider<AppState>.value(
      value: state,
      child: Consumer<AppState>(
        builder: (context, s, _) => MaterialApp(
          onGenerateTitle: (ctx) => AppLocalizations.of(ctx).appTitle,
          debugShowCheckedModeBanner: false,
          themeMode: ThemeMode.dark,
          theme: ThemeData(colorSchemeSeed: seed, useMaterial3: true),
          darkTheme: ThemeData(
            colorSchemeSeed: seed,
            brightness: Brightness.dark,
            useMaterial3: true,
          ),
          locale: s.locale,
          supportedLocales: AppLocalizations.supportedLocales,
          // material_ui's delegates (Material, Cupertino, Widgets): the
          // generated list still names the deprecated flutter_localizations
          // ones, which material_ui widgets don't read.
          localizationsDelegates: const [
            AppLocalizations.delegate,
            ...GlobalMaterialLocalizations.delegates,
          ],
          localeListResolutionCallback: (preferred, supported) {
            final resolved = s.locale ?? resolveLocale(preferred, supported);
            s.useEffectiveLocale(resolved);
            return resolved;
          },
          home: s.onAndroid
              ? const AndroidNotice()
              : HomeShell(guardWindowClose: guardWindowClose),
        ),
      ),
    );
  }
}
