import 'package:flutter/widgets.dart';

/// A language Thimsy supports: its Thimsy code (what the engine and the
/// device use), the Flutter locale, and its name in that language.
class Language {
  final String code;
  final Locale locale;
  final String nativeName;
  const Language(this.code, this.locale, this.nativeName);
}

/// English plus the 21 languages Thimsy translates into, by Thimsy code.
const languages = <Language>[
  Language('en', Locale('en'), 'English'),
  Language('ar', Locale('ar'), 'العربية'),
  Language('da', Locale('da'), 'Dansk'),
  Language('de', Locale('de'), 'Deutsch'),
  Language('el', Locale('el'), 'Ελληνικά'),
  Language('es', Locale('es'), 'Español'),
  Language('fi', Locale('fi'), 'Suomi'),
  Language('fr', Locale('fr'), 'Français'),
  Language('he', Locale('he'), 'עברית'),
  Language('it', Locale('it'), 'Italiano'),
  Language('ja', Locale('ja'), '日本語'),
  Language('ko', Locale('ko'), '한국어'),
  Language('nb', Locale('nb'), 'Norsk bokmål'),
  Language('nl', Locale('nl'), 'Nederlands'),
  Language('pl', Locale('pl'), 'Polski'),
  Language('pt', Locale('pt'), 'Português'),
  Language('ru', Locale('ru'), 'Русский'),
  Language('sv', Locale('sv'), 'Svenska'),
  Language('th', Locale('th'), 'ไทย'),
  Language('tr', Locale('tr'), 'Türkçe'),
  Language('zh-cn', Locale('zh'), '简体中文'),
  Language(
    'zh-tw',
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hant'),
    '繁體中文',
  ),
];

Language? languageByCode(String? code) {
  for (final l in languages) {
    if (l.code == code) return l;
  }
  return null;
}

/// Maps any locale (system or chosen) to the closest Thimsy language,
/// English when none fits. Traditional Chinese is picked for the Hant
/// script or for Taiwan, Hong Kong and Macau.
Language languageForLocale(Locale? locale) {
  if (locale == null) return languages.first;
  final lc = locale.languageCode.toLowerCase();
  if (lc == 'zh') {
    final hant =
        locale.scriptCode == 'Hant' ||
        (locale.scriptCode == null &&
            const {'TW', 'HK', 'MO'}.contains(locale.countryCode));
    return languageByCode(hant ? 'zh-tw' : 'zh-cn')!;
  }
  // Norwegian: "no" and "nn" fall back to bokmål.
  final code = (lc == 'no' || lc == 'nn') ? 'nb' : (lc == 'iw' ? 'he' : lc);
  return languageByCode(code) ?? languages.first;
}

/// `MaterialApp.localeListResolutionCallback`: first system locale Thimsy
/// speaks, else English.
Locale resolveLocale(List<Locale>? preferred, Iterable<Locale> supported) {
  for (final l in preferred ?? const <Locale>[]) {
    final lang = languageForLocale(l);
    if (lang.code != 'en' || l.languageCode == 'en') return lang.locale;
  }
  return languages.first.locale;
}
