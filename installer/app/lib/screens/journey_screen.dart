import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../engine/engine.dart';
import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class JourneyScreen extends StatelessWidget {
  const JourneyScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final journeys = s.plan?.journeys ?? Journey.defaults;
    final order = [
      'first_time',
      'returning',
    ].where(journeys.containsKey).toList();
    return StepPage(
      title: l.journeyTitle,
      subtitle: l.journeySubtitle,
      children: [
        Wrap(
          spacing: 16,
          runSpacing: 16,
          children: [
            for (final id in order)
              _JourneyCard(
                journey: journeys[id]!,
                selected: s.journeyId == id,
                onTap: () => s.selectJourney(id),
              ),
          ],
        ),
      ],
    );
  }
}

class _JourneyCard extends StatelessWidget {
  final Journey journey;
  final bool selected;
  final VoidCallback onTap;
  const _JourneyCard({
    required this.journey,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final first = journey.id == 'first_time';
    final title = journey.title.isNotEmpty
        ? journey.title
        : (first ? l.journeyFirstTitle : l.journeyReturningTitle);
    final body = journey.body.isNotEmpty
        ? journey.body
        : (first ? l.journeyFirstBody : l.journeyReturningBody);
    return SizedBox(
      width: 340,
      child: Card(
        key: ValueKey('journey-${journey.id}'),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(
            color: selected ? scheme.primary : scheme.outlineVariant,
            width: selected ? 2 : 1,
          ),
        ),
        child: InkWell(
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(
                  journey.unlock ? Icons.lock_open : Icons.system_update,
                  size: 36,
                  color: scheme.primary,
                ),
                const SizedBox(height: 12),
                Text(title, style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 6),
                Text(body),
                const SizedBox(height: 8),
                Text(
                  journey.unlock
                      ? l.journeyIncludesUnlock
                      : l.journeySkipsUnlock,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
