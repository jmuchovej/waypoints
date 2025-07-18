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

  Future<String> dataDirectory() async {
    final base = await getApplicationDocumentsDirectory();
    final name = switch (_appFlavor) {
      AppFlavor.development => 'dev.tempo',
      AppFlavor.staging => 'dev.tempo',
      AppFlavor.production => 'do.tempo',
    };
    return p.join(base.path, name);
  }

  bool get isDevelopment => _appFlavor == AppFlavor.development && kDebugMode;
  bool get isStaging => _appFlavor == AppFlavor.staging;
  bool get isProduction => _appFlavor == AppFlavor.production;
}
