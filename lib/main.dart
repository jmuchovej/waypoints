import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/devtools/database/seed.dart';
import 'flavors.dart';
import 'src/infrastructure/logging.dart';
import 'src/objectbox/objectbox.dart';
import 'src/objectbox/providers.dart';
import 'src/l10n/l10n.dart';
import 'src/router.dart';
import 'src/theme.dart';

void main() async {
  F.appFlavor = Flavor.values.firstWhere(
    (element) => element.name == appFlavor,
  );

  WidgetsFlutterBinding.ensureInitialized();
  final obx = await ObjectBox.create();

  // Create a temporary container to get the UUID repository
  final container = ProviderContainer(
    overrides: [objectBoxProvider.overrideWithValue(obx)],
  );

  // Seed with polymorphic relationships in development
  if (F.appFlavor == Flavor.development) {
    final seedService = SeedDataService.fromContainer(container);
    await seedService.seedIfNeeded();
    // Optional: Enable other debug flags
    // debugPaintSizeEnabled = true;
    // debugPaintBaselinesEnabled = true; // Shows text baselines
    // debugPaintPointersEnabled = true; // Shows pointer hit testing
    // debugPaintLayerBordersEnabled = true; // Shows layer boundaries
  }
  initLoggers(Level.FINEST, tempoLoggers);
  initLoggers(Level.WARNING, superEditorLoggers);
  deactivateLoggers(FTLLoggers);

  runApp(UncontrolledProviderScope(container: container, child: Tempo()));
}

class Tempo extends ConsumerWidget {
  Tempo({super.key});

  // Create the auto_route router
  final _appRouter = AppRouter();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final textTheme = createTextTheme(context, display: 'Lato', body: 'Lato');
    final theme = TempoTheme(textTheme);

    tempoLog.info('🚀 Tempo.build() called - creating MaterialApp.router');
    return MaterialApp.router(
      title: F.title,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: theme.light(),
      darkTheme: theme.dark(),
      themeMode: ThemeMode.system,
      routerConfig: _appRouter.config(),
      debugShowCheckedModeBanner: false,
    );
  }
}
