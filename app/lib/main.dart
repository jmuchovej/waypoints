import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'flavors.dart';
import 'src/devtools/database/seed.dart';
import 'src/generated/rust/frb_generated.dart';
import 'src/infrastructure/logging.dart';
import 'src/l10n/l10n.dart';
import 'src/objectbox/objectbox.dart';
import 'src/router.dart';
import 'src/styling/stylesheet.dart';
import 'src/styling/text_styles.dart';
import 'src/styling/theme.dart';

Future<void> main() async {
  await RustLib.init();

  WidgetsFlutterBinding.ensureInitialized();

  // Initialize logging configuration
  LoggerConfig.initialize();

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

  runApp(UncontrolledProviderScope(container: container, child: Waypoints()));
}

class Waypoints extends ConsumerWidget {
  const Waypoints({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final textTheme = createTextTheme(context, display: 'Lato', body: 'Lato');
    final theme = WaypointsTheme(textTheme);

    final flavor = ref.read(flavorProvider);
    final router = ref.read(routerProvider);

    return MaterialApp.router(
      title: flavor.title,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: theme.light(),
      darkTheme: theme.dark(),
      themeMode: ThemeMode.system,
      routerConfig: router,
      debugShowCheckedModeBanner: false,
      builder: (context, child) {
        // Provide WaypointsTextStyles to all widgets via ProviderScope override
        return ProviderScope(
          overrides: [
            waypointsTextStylesProvider.overrideWithValue(
              WaypointsTextStyles.fromContext(context),
            ),
            stylesheetProvider.overrideWithValue(
              WaypointsStylesheet.fromContext(context),
            ),
          ],
          child: child!,
        );
      },
    );
  }
}
