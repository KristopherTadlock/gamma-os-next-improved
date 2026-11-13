import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:thimsy_installer/l10n/app_localizations.dart';
import 'package:thimsy_installer/l10n/languages.dart';

Map<String, dynamic> readArb(File f) =>
    jsonDecode(f.readAsStringSync()) as Map<String, dynamic>;

Set<String> messageKeys(Map<String, dynamic> arb) =>
    arb.keys.where((k) => !k.startsWith('@')).toSet();

Set<String> placeholdersIn(String message) =>
    RegExp(r'\{(\w+)\}').allMatches(message).map((m) => m.group(1)!).toSet();

void main() {
  final dir = Directory('lib/l10n');
  final en = readArb(File('lib/l10n/app_en.arb'));
  final enKeys = messageKeys(en);
  final arbs = dir
      .listSync()
      .whereType<File>()
      .where((f) => f.path.endsWith('.arb'))
      .toList();

  test('there is an ARB for English plus the 21 Thimsy languages', () {
    expect(arbs, hasLength(22));
    final locales = arbs.map((f) => readArb(f)['@@locale']).toSet();
    expect(locales, {
      'en',
      'ar',
      'da',
      'de',
      'el',
      'es',
      'fi',
      'fr',
      'he',
      'it',
      'ja',
      'ko',
      'nb',
      'nl',
      'pl',
      'pt',
      'ru',
      'sv',
      'th',
      'tr',
      'zh',
      'zh_Hant',
    });
  });

  for (final f in arbs) {
    final name = f.uri.pathSegments.last;
    test('$name has exactly the keys and placeholders of app_en.arb', () {
      final arb = readArb(f);
      expect(messageKeys(arb), enKeys, reason: 'keys differ in $name');
      for (final k in enKeys) {
        final want = placeholdersIn(en[k] as String);
        expect(
          placeholdersIn(arb[k] as String),
          want,
          reason: 'placeholders of $k in $name',
        );
        final declared =
            ((en['@$k'] as Map?)?['placeholders'] as Map?)?.keys.toSet() ??
            <String>{};
        expect(declared, want, reason: 'placeholder metadata of $k');
        expect((arb[k] as String).trim(), isNotEmpty, reason: '$k in $name');
      }
    });
  }

  // Texts that name another part of the UI use that part's own label, in
  // every language: a renamed or missing button shows up here.
  group('texts point at UI that exists', () {
    const references = {
      'errEngineCrashedHint': ['copyDetails'],
      'finishOnWindowsBody': ['imageSourceFiles'],
      'connectLaterBody': ['stepReview'],
      'fullInstallNeededBody': ['stepReview'],
      'downloadModeAgain': ['runAgain'],
    };
    for (final f in arbs) {
      final name = f.uri.pathSegments.last;
      test(name, () {
        final arb = readArb(f);
        for (final MapEntry(key: k, value: labels) in references.entries) {
          for (final label in labels) {
            expect(
              arb[k] as String,
              contains(arb[label] as String),
              reason: '$k in $name should name "${arb[label]}" ($label)',
            );
          }
        }
      });
    }

    test('no English text names UI that was removed or never existed', () {
      final all = enKeys.map((k) => en[k] as String).join('\n');
      for (final gone in [
        'Already downloaded files', // the import option's old name
        'diagnostic log', // there is no "save log"; Copy details is
        'Press Next to start installing', // Next on Connect opens Review
        'Copy the result to a Windows PC', // nothing there can import it
      ]) {
        expect(all, isNot(contains(gone)), reason: gone);
      }
    });
  });

  test('every Thimsy language maps to a supported Flutter locale', () {
    final supported = AppLocalizations.supportedLocales.toSet();
    expect(languages, hasLength(22));
    for (final l in languages) {
      expect(supported, contains(l.locale), reason: l.code);
    }
  });

  test('system locales resolve to the right Thimsy language', () {
    expect(languageForLocale(const Locale('zh', 'CN')).code, 'zh-cn');
    expect(languageForLocale(const Locale('zh', 'TW')).code, 'zh-tw');
    expect(languageForLocale(const Locale('zh', 'HK')).code, 'zh-tw');
    expect(
      languageForLocale(
        const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hant'),
      ).code,
      'zh-tw',
    );
    expect(languageForLocale(const Locale('pt', 'BR')).code, 'pt');
    expect(languageForLocale(const Locale('no')).code, 'nb');
    expect(languageForLocale(const Locale('xx')).code, 'en');
    expect(
      resolveLocale(const [Locale('xx'), Locale('fr', 'CA')], const []),
      const Locale('fr'),
    );
    expect(resolveLocale(const [Locale('xx')], const []), const Locale('en'));
  });
}
