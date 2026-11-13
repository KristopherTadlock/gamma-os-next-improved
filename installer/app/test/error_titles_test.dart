// Every error code the engine can send has a translated title. The codes
// are read from the engine's own source (the `Code` enum, serialized in
// snake_case), so a code added there without a title here fails.
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:thimsy_installer/l10n/app_localizations.dart';
import 'package:thimsy_installer/widgets/common.dart';

const errorRs = '../engine/crates/thimsy-engine/src/error.rs';

/// `DeviceNotFound` → `device_not_found` (serde `rename_all = "snake_case"`).
String snake(String camel) => camel
    .replaceAllMapped(RegExp(r'(?<=[a-z0-9])([A-Z])'), (m) => '_${m[1]}')
    .toLowerCase();

/// The codes `error.rs` defines: the `Code` enum's variants, and the
/// strings `Code::as_str` returns (they must agree).
({Set<String> variants, Set<String> strings}) engineCodes(String source) {
  final body = RegExp(r'pub enum Code\s*\{([^}]*)\}')
      .firstMatch(source)
      ?.group(1);
  if (body == null) fail('no `pub enum Code { … }` in $errorRs');
  final variants = {
    for (final line in body.split('\n'))
      if (RegExp(r'^\s*([A-Z]\w*)\s*,?\s*$').firstMatch(line) case final m?)
        snake(m[1]!),
  };
  final strings = {
    for (final m in RegExp(r'Code::\w+\s*=>\s*"([a-z_]+)"').allMatches(source))
      m[1]!,
  };
  return (variants: variants, strings: strings);
}

void main() {
  final file = File(errorRs);

  test('error.rs is where the test expects it', () {
    expect(
      file.existsSync(),
      isTrue,
      reason: '$errorRs (relative to installer/app) is missing',
    );
  });

  final source = file.existsSync() ? file.readAsStringSync() : '';
  final codes = engineCodes(source);
  // Sent by the engine (docs/engine-protocol.md) even before error.rs has
  // them here: checked either way.
  const alsoSent = {'busy', 'full_install_needed', 'device_not_found'};
  final all = {...codes.variants, ...codes.strings, ...alsoSent};

  test('the parser finds the codes, and the enum and as_str agree', () {
    expect(codes.variants, containsAll(['device_not_found', 'internal']));
    expect(codes.variants.length, greaterThanOrEqualTo(13));
    if (codes.strings.isNotEmpty) expect(codes.strings, codes.variants);
    expect(snake('FullInstallNeeded'), 'full_install_needed');
  });

  for (final locale in AppLocalizations.supportedLocales) {
    test('every engine error code has a title in $locale', () {
      final l = lookupAppLocalizations(locale);
      for (final code in all) {
        final title = errorTitleFor(l, code);
        expect(title, isNotNull, reason: 'no title for $code');
        expect(title!.trim(), isNotEmpty, reason: '$code in $locale');
        expect(title, isNot(l.errTitleUnknown), reason: code);
      }
    });
  }
}
