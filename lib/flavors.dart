enum Flavor { development, staging, production }

class F {
  static late final Flavor appFlavor;

  static String get name => appFlavor.name;

  static String get title {
    switch (appFlavor) {
      case Flavor.development:
        return 'Tempo (Dev)';
      case Flavor.staging:
        return 'Tempo (Staging)';
      case Flavor.production:
        return 'Tempo';
    }
  }
}
