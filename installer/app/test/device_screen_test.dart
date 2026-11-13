import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:thimsy_installer/engine/engine.dart';
import 'package:thimsy_installer/engine/fake_engine.dart';
import 'package:thimsy_installer/main.dart';
import 'package:thimsy_installer/state/app_state.dart';
import 'package:thimsy_installer/state/device_search.dart';

Finder key(String k) => find.byKey(ValueKey(k));

Future<AppState> start(
  WidgetTester tester, {
  String host = 'linux',
  Size size = const Size(1600, 1400),
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final state = AppState(
    FakeEngine(step: Duration.zero, host: host),
    onAndroid: false,
  );
  await tester.pumpWidget(ThimsyInstallerApp(state: state));
  await tester.pumpAndSettle();
  return state;
}

Future<void> tap(WidgetTester tester, Finder f) async {
  await tester.ensureVisible(f);
  await tester.pumpAndSettle();
  await tester.tap(f);
  await tester.pumpAndSettle();
}

Future<void> search(WidgetTester tester, String text) async {
  await tester.enterText(key('device-search'), text);
  await tester.pumpAndSettle();
}

Finder get dropdownField => find.descendant(
  of: key('device-dropdown'),
  matching: find.byType(TextField),
);

String dropdownText(WidgetTester tester) =>
    tester.widget<TextField>(dropdownField).controller!.text;

Finder menuItem(String label) => find.widgetWithText(MenuItemButton, label);

/// Opens the dropdown, optionally types [filter], and picks [label].
Future<void> pickInDropdown(
  WidgetTester tester,
  String label, {
  String? filter,
}) async {
  await tap(tester, dropdownField);
  if (filter != null) {
    await tester.enterText(dropdownField, filter);
    await tester.pumpAndSettle();
  }
  await tap(tester, menuItem(label));
}

Set<String> shownDeviceIds(WidgetTester tester) => {
  for (final w in tester.widgetList<Card>(find.byType(Card)))
    if (w.key case ValueKey<String>(:final value)
        when value.startsWith('device-'))
      value.substring('device-'.length),
};

DeviceInfo dev(String name, String maker, String soc) => DeviceInfo(
  id: name,
  name: name,
  maker: maker,
  soc: soc,
  display: '',
  status: 'candidate',
  build: 'none',
  boot: 'none',
  install: const {},
);

void main() {
  group('device matching', () {
    final cube = dev('Anbernic RG Cube', 'Anbernic', 'Unisoc T820');
    final rp5 = dev('Retroid Pocket 5', 'Retroid', 'Qualcomm Snapdragon 865');

    test('ignores case, spaces and punctuation', () {
      for (final q in ['rg cube', 'rgcube', 'RG-Cube', ' R.G. cube ', 'CUBE']) {
        expect(deviceMatches(cube, q), isTrue, reason: q);
      }
      expect(normalizeForSearch('RG-Cube 3.0'), 'rgcube30');
    });

    test('matches maker and SoC, and words across fields', () {
      expect(deviceMatches(cube, 't820'), isTrue);
      expect(deviceMatches(cube, 'unisoc'), isTrue);
      expect(deviceMatches(rp5, 'snapdragon 865'), isTrue);
      expect(deviceMatches(rp5, 'retroid 865'), isTrue);
      expect(deviceMatches(cube, 'anbernic 865'), isFalse);
      expect(deviceMatches(cube, 'pocket'), isFalse);
      expect(deviceMatches(cube, ''), isTrue);
      expect(deviceMatches(cube, ' - '), isTrue);
    });

    test('groups by maker, then name, makerless last', () {
      final groups = groupByMaker([
        rp5,
        dev('Mystery', '', 'X'),
        dev('Anbernic RG557', 'Anbernic', 'D8300'),
        cube,
        dev('AYANEO Pocket Micro', 'AYANEO', 'G99'),
      ]);
      expect(groups.map((g) => g.key), ['Anbernic', 'AYANEO', 'Retroid', '']);
      expect(groups.first.value.map((d) => d.name), [
        'Anbernic RG Cube',
        'Anbernic RG557',
      ]);
    });
  });

  testWidgets('devices are grouped under maker headings', (tester) async {
    await start(tester);
    final makers = ['Anbernic', 'AYANEO', 'GameMT', 'MANGMI', 'Retroid'];
    final tops = [
      for (final m in makers) tester.getTopLeft(key('maker-$m')).dy,
    ];
    expect(tops, [...tops]..sort());
    expect(shownDeviceIds(tester), hasLength(12));
  });

  testWidgets('search filters by name ignoring spaces and punctuation', (
    tester,
  ) async {
    await start(tester);
    for (final q in ['rg cube', 'rgcube', 'RG-Cube']) {
      await search(tester, q);
      expect(shownDeviceIds(tester), {'anbernic-rg-cube'}, reason: q);
      expect(key('maker-Anbernic'), findsOneWidget);
      expect(key('maker-Retroid'), findsNothing);
    }
    await search(tester, 'pocket');
    expect(shownDeviceIds(tester), {
      'retroid-pocket-4-pro',
      'retroid-pocket-5',
      'retroid-pocket-mini',
      'retroid-pocket-classic',
      'ayaneo-pocket-micro',
    });
  });

  testWidgets('search matches the SoC and the maker', (tester) async {
    await start(tester);
    await search(tester, 't820');
    expect(shownDeviceIds(tester), {'anbernic-rg-cube', 'anbernic-rg406h'});
    await search(tester, 'Snapdragon 865');
    expect(shownDeviceIds(tester), {
      'retroid-pocket-5',
      'retroid-pocket-mini',
      'mangmi-air-x',
    });
    await search(tester, 'mangmi');
    expect(shownDeviceIds(tester), {'mangmi-air-x'});
  });

  testWidgets('no matches: friendly message, and clearing brings all back', (
    tester,
  ) async {
    await start(tester);
    await search(tester, 'zzz');
    expect(key('device-no-results'), findsOneWidget);
    expect(find.text('No devices match “zzz”'), findsOneWidget);
    expect(shownDeviceIds(tester), isEmpty);

    await tap(tester, key('device-no-results-clear'));
    expect(key('device-no-results'), findsNothing);
    expect(shownDeviceIds(tester), hasLength(12));
    expect(
      tester.widget<TextField>(key('device-search')).controller!.text,
      isEmpty,
    );

    // The clear button inside the search box does the same.
    await search(tester, 'qwerty');
    expect(key('device-no-results'), findsOneWidget);
    await tap(tester, key('device-search-clear'));
    expect(shownDeviceIds(tester), hasLength(12));
    expect(key('device-search-clear'), findsNothing);
  });

  testWidgets('works-here filter with nothing left offers to show all', (
    tester,
  ) async {
    await start(tester, host: 'android');
    await tap(tester, key('works-here'));
    expect(key('device-no-results'), findsOneWidget);
    expect(
      find.text("This computer can't install any of these devices"),
      findsOneWidget,
    );
    await tap(tester, key('device-show-all'));
    expect(shownDeviceIds(tester), hasLength(12));
  });

  testWidgets('dropdown selects the device and highlights its card', (
    tester,
  ) async {
    final state = await start(tester);
    // The dropdown lists every device, sorted by maker then name.
    await tap(tester, dropdownField);
    expect(find.byType(MenuItemButton), findsNWidgets(12));
    await tap(tester, menuItem('Retroid Pocket Mini'));

    expect(state.deviceId, 'retroid-pocket-mini');
    expect(key('selected-retroid-pocket-mini'), findsOneWidget);
    expect(
      find.byWidgetPredicate(
        (w) =>
            w.key is ValueKey<String> &&
            (w.key! as ValueKey<String>).value.startsWith('selected-'),
      ),
      findsOneWidget,
    );
    expect(dropdownText(tester), 'Retroid Pocket Mini');
    // The selected device's notes show as before.
    expect(find.text('• Not buildable yet.'), findsOneWidget);

    // Typing in the dropdown filters it the same forgiving way.
    await pickInDropdown(tester, 'Anbernic RG Cube', filter: 'rg-cube');
    expect(state.deviceId, 'anbernic-rg-cube');
    expect(key('selected-anbernic-rg-cube'), findsOneWidget);
    expect(key('selected-retroid-pocket-mini'), findsNothing);
  });

  testWidgets('dropdown filter uses the SoC too', (tester) async {
    await start(tester);
    await tap(tester, dropdownField);
    await tester.enterText(dropdownField, 't820');
    await tester.pumpAndSettle();
    expect(find.byType(MenuItemButton), findsNWidgets(2));
    expect(menuItem('Anbernic RG406H'), findsOneWidget);
  });

  testWidgets('tapping a card updates the dropdown', (tester) async {
    final state = await start(tester);
    expect(dropdownText(tester), isEmpty);
    await tap(tester, key('device-anbernic-rg557'));
    expect(state.deviceId, 'anbernic-rg557');
    expect(dropdownText(tester), 'Anbernic RG557');

    // Even after a half-typed filter in the dropdown.
    await tap(tester, dropdownField);
    await tester.enterText(dropdownField, 'retro');
    await tester.pumpAndSettle();
    await tester.tapAt(const Offset(1500, 1300)); // close the menu
    await tester.pumpAndSettle();
    await tap(tester, key('device-gamemt-e6-max'));
    expect(dropdownText(tester), 'GameMT E6 MAX');
    expect(key('selected-gamemt-e6-max'), findsOneWidget);
  });

  testWidgets('dropdown pick hidden by the search clears the search', (
    tester,
  ) async {
    final state = await start(tester);
    await search(tester, 't820');
    expect(key('device-mangmi-air-x'), findsNothing);
    await pickInDropdown(tester, 'MANGMI Air X');
    expect(state.deviceId, 'mangmi-air-x');
    expect(
      tester.widget<TextField>(key('device-search')).controller!.text,
      isEmpty,
    );
    expect(key('selected-mangmi-air-x'), findsOneWidget);

    // A pick the search already shows keeps the search.
    await search(tester, 't820');
    await pickInDropdown(tester, 'Anbernic RG406H');
    expect(
      tester.widget<TextField>(key('device-search')).controller!.text,
      't820',
    );
  });

  testWidgets('a device picked in the dropdown goes on to the profile step', (
    tester,
  ) async {
    await start(tester);
    await pickInDropdown(tester, 'Anbernic RG Cube');
    await tap(tester, key('next'));
    expect(find.text('Choose an experience'), findsOneWidget);
  });

  testWidgets('Enter in the search picks a single match, then Next works', (
    tester,
  ) async {
    final state = await start(tester);
    await search(tester, 'rg cube');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();
    expect(state.deviceId, 'anbernic-rg-cube');
    expect(dropdownText(tester), 'Anbernic RG Cube');
    await tap(tester, key('next'));
    expect(find.text('Choose an experience'), findsOneWidget);
  });

  testWidgets('a Windows-only device picked in the dropdown still says so', (
    tester,
  ) async {
    await start(tester, host: 'macos');
    expect(key('needs-windows-anbernic-rg405m'), findsOneWidget);
    await pickInDropdown(tester, 'Anbernic RG405M', filter: 'rg405');
    await tap(tester, key('next'));
    expect(key('needs-windows-dialog'), findsOneWidget);
    expect(
      find.textContaining('flashed with a Windows-only tool'),
      findsOneWidget,
    );
    // Not buildable yet, so no export.
    expect(find.text('Build and export anyway'), findsNothing);
  });

  testWidgets('a device with a block shows it', (tester) async {
    await start(tester);
    await search(tester, 'classic');
    await tap(tester, key('device-retroid-pocket-classic'));
    expect(key('device-details'), findsOneWidget);
    expect(
      find.text('Firmware 1.0.0.5 cannot be unlocked yet.'),
      findsOneWidget,
    );
  });

  testWidgets('small window: stacked controls, still searchable', (
    tester,
  ) async {
    final state = await start(tester, size: const Size(800, 600));
    final searchBox = tester.getTopLeft(key('device-search'));
    final dropdown = tester.getTopLeft(dropdownField);
    expect(dropdown.dy, greaterThan(searchBox.dy));
    await search(tester, 'gamemt');
    expect(shownDeviceIds(tester), {'gamemt-e6-max'});
    await pickInDropdown(tester, 'Retroid Pocket 5', filter: 'pocket 5');
    expect(state.deviceId, 'retroid-pocket-5');
    expect(key('selected-retroid-pocket-5'), findsOneWidget);
  });
}
