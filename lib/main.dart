import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:tempo/app.dart';
import 'package:tempo/flavors.dart';
import 'package:tempo/main_development.dart' as d;
import 'package:tempo/main_production.dart' as p;
import 'package:tempo/objectbox.dart';

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

  runApp(ProviderScope(child: Tempo(routes: routes)));
}
