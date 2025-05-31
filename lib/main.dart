import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:tempo/dev/database/seed.dart';
import 'package:tempo/flavors.dart';
import 'package:tempo/theme/theme.dart';
import 'package:tempo/src/database/objectbox.dart';
import 'package:tempo/src/infrastructure.dart';
import 'package:tempo/src/l10n.dart';

late ObjectBox obx;

void main() async {
  F.appFlavor = Flavor.values.firstWhere(
    (element) => element.name == appFlavor,
  );

  WidgetsFlutterBinding.ensureInitialized();
  obx = await ObjectBox.create();

  if (F.appFlavor == Flavor.development) {
    await SeedDataService.seedIfNeeded();
    // Optional: Enable other debug flags
    // debugPaintSizeEnabled = true;
    // debugPaintBaselinesEnabled = true; // Shows text baselines
    // debugPaintPointersEnabled = true; // Shows pointer hit testing
    // debugPaintLayerBordersEnabled = true; // Shows layer boundaries
  }
  initLoggers(Level.FINEST, tempoLoggers);
  initLoggers(Level.FINER, superEditorLoggers);
  deactivateLoggers(FTLLoggers);

  runApp(const ProviderScope(child: Tempo()));
}

class Tempo extends StatelessWidget {
  const Tempo({super.key});

  @override
  Widget build(BuildContext context) {
    final textTheme = createTextTheme(
      context,
      display: "Lato",
      body: "Lato",
    );
    final theme = TempoTheme(textTheme);

    return MaterialApp.router(
      title: F.title,
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
