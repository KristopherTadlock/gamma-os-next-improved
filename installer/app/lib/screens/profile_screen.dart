import 'package:material_ui/material_ui.dart';
import 'package:provider/provider.dart';

import '../engine/engine.dart';
import '../l10n/app_localizations.dart';
import '../state/app_state.dart';
import '../widgets/common.dart';

class ProfileScreen extends StatelessWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = context.watch<AppState>();
    final cat = s.catalog!;
    final plan = s.plan;
    return StepPage(
      title: l.profileTitle,
      subtitle: l.profileSubtitle,
      children: [
        if (s.planError != null)
          ErrorNotice(
            title: l.planError,
            error: errorEventOf(s.planError!),
            action: OutlinedButton(
              onPressed: s.loadPlans,
              child: Text(l.tryAgain),
            ),
          ),
        if (s.plansLoading) const LinearProgressIndicator(),
        Wrap(
          spacing: 16,
          runSpacing: 16,
          children: [
            for (final p in cat.profiles)
              _ProfileCard(
                profile: p,
                plan: s.plans[p.id],
                selected: p.id == s.profileId,
                onTap: () => s.selectProfile(p.id),
              ),
          ],
        ),
        if (plan != null && plan.available) ...[
          const SizedBox(height: 24),
          _PlanDetails(plan),
        ],
      ],
    );
  }
}

class _ProfileCard extends StatelessWidget {
  final ProfileInfo profile;
  final Plan? plan;
  final bool selected;
  final VoidCallback onTap;
  const _ProfileCard({
    required this.profile,
    required this.plan,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final available = profile.available && (plan?.available ?? false);
    return SizedBox(
      width: 260,
      child: Opacity(
        opacity: available ? 1 : 0.55,
        child: Card(
          key: ValueKey('profile-${profile.id}'),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
            side: BorderSide(
              color: selected ? scheme.primary : scheme.outlineVariant,
              width: selected ? 2 : 1,
            ),
          ),
          child: InkWell(
            onTap: available ? onTap : null,
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    profile.name,
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 6),
                  Text(profile.summary),
                  const SizedBox(height: 10),
                  if (!available)
                    Text(
                      l.notPublished,
                      style: TextStyle(color: scheme.tertiary),
                    )
                  else if (plan != null)
                    Text(
                      l.profileSizes(
                        formatBytes(plan!.downloadBytes),
                        formatBytes(plan!.diskBytes),
                      ),
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _PlanDetails extends StatelessWidget {
  final Plan plan;
  const _PlanDetails(this.plan);

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).textTheme;
    Widget section(String title, List<Widget> rows) => Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(title, style: t.titleMedium),
        const SizedBox(height: 4),
        ...rows,
        const SizedBox(height: 16),
      ],
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        section(l.emulators, [
          for (final e in plan.emulators.where((e) => e.included))
            Text('• ${e.name}${e.detail == null ? '' : ' — ${e.detail}'}'),
        ]),
        if (plan.apps.isNotEmpty)
          section(l.apps, [
            for (final a in plan.apps.where((a) => a.included))
              Text('• ${a.name}'),
          ]),
        if (plan.excluded.isNotEmpty)
          section(l.notIncluded, [
            for (final x in plan.excluded)
              Text('• ${x.name}${x.reason == null ? '' : ': ${x.reason}'}'),
          ]),
      ],
    );
  }
}
