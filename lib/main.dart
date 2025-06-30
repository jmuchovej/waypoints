import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'flavors.dart';
import 'src/devtools/database/seed.dart';
import 'src/infrastructure/logging.dart';
import 'src/l10n/l10n.dart';
import 'src/objectbox/objectbox.dart';
import 'src/router_go.dart';
import 'src/theme.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final flavor = Flavor(appFlavor);
  final obx = await ObjectBox.create(flavor);

  // Create a temporary container to get the UUID repository
  final container = ProviderContainer(
    overrides: [
      objectBoxProvider.overrideWithValue(obx),
      flavorProvider.overrideWithValue(flavor),
    ],
  );

  // Seed with polymorphic relationships in development
  if (flavor.isDevelopment) {
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

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final textTheme = createTextTheme(context, display: 'Lato', body: 'Lato');
    final theme = TempoTheme(textTheme);

    final flavor = ref.read(flavorProvider);
    final router = ref.read(routerProvider);

    tempoLog.info('🚀 Tempo.build() called - creating MaterialApp.router');
    return MaterialApp.router(
      title: flavor.title,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: theme.light(),
      darkTheme: theme.dark(),
      themeMode: ThemeMode.system,
      routerConfig: router,
      debugShowCheckedModeBanner: false,
    );
  }
}
