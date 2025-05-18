import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flux/app.dart';
import 'package:flux/flavors.dart';
import 'package:flux/main_development.dart' as d;
import 'package:flux/main_production.dart' as p;
import 'package:flux/objectbox.dart';

void main() async {
  F.appFlavor = Flavor.values.firstWhere(
    (element) => element.name == appFlavor,
  );

  WidgetsFlutterBinding.ensureInitialized();
  obx = await ObjectBox.create();

  switch (F.appFlavor) {
    case Flavor.development:
      await d.preRunApp();
      break;
    case Flavor.staging:
    case Flavor.production:
      await p.preRunApp();
      break;
  }

  List<AppRoute> routes;

  switch (F.appFlavor) {
    case Flavor.development:
      routes = d.appRoutes();
      break;
    case Flavor.staging:
    case Flavor.production:
      routes = p.appRoutes();
      break;
  }

  runApp(ProviderScope(child: TempoFlux(routes: routes)));
}
