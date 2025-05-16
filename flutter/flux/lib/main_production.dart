import 'package:flutter/material.dart';
import 'package:flux/config/environment.dart';
import 'package:flux/objectbox.dart';
import 'package:flux/app.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  EnvironmentConfig.setEnvironment(Environment.production);

  await ObjectBox.create();

  final routes = [...defaultAppRoutes];

  runApp(TempoFlux(routes: routes));
}
