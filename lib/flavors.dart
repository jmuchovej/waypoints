import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:path/path.dart' as p;
import 'package:flutter_riverpod/flutter_riverpod.dart';

final flavorProvider = Provider<Flavor>((ref) {
  throw UnimplementedError("Flavor must be initialized before use");
});

enum AppFlavor { development, staging, production }

class Flavor {
  Flavor(String? appFlavor) {
    appFlavor ??= AppFlavor.development.name;
    _appFlavor = AppFlavor.values.firstWhere((e) => e.name == appFlavor);
  }
  late final AppFlavor _appFlavor;

  String get name => _appFlavor.name;

  String get title {
    switch (_appFlavor) {
      case AppFlavor.development:
        return 'Tempo (Dev)';
      case AppFlavor.staging:
        return 'Tempo (Staging)';
      case AppFlavor.production:
        return 'Tempo';
    }
  }

  String get bundleID => switch(_appFlavor) {
    AppFlavor.development => "com.tempo.dev",
    AppFlavor.staging => "com.tempo.staging",
    AppFlavor.production => "com.tempo",
  };

  bool get isDevelopment => _appFlavor == AppFlavor.development && kDebugMode;
  bool get isStaging => _appFlavor == AppFlavor.staging;
  bool get isProduction => _appFlavor == AppFlavor.production;

  Future<Directory> get dataDirectory async {
    final base = await getApplicationSupportDirectory();
    return Directory(p.join(base.path, bundleID));
  }

  Future<Directory> get assetsDirectory async {
    final base = await getApplicationSupportDirectory();
    return Directory(p.join(base.path, bundleID));
  }
}

class TempoData {
  const TempoData(this.flavor);

  final Flavor flavor;

  Future<Directory> get root async {
    final base = await getApplicationDocumentsDirectory();
    final directory = Directory(p.join(base.path, flavor.bundleID));
    return directory.create();
  }

  Future<Directory> get appData async {
    final directory = Directory(p.join((await root).path, ".tempo"));
    return directory.create();
  }

  Future<Directory> get database async {
    final directory = Directory(p.join((await appData).path, "database"));
    return directory.create();
  }

  Future<Directory> get assets async {
    final directory = Directory(p.join((await root).path, "assets"));
    return directory.create();
  }
}
