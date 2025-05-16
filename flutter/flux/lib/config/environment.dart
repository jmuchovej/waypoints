enum Environment { development, staging, production }

class EnvironmentConfig {
  static late Environment _environment;

  static void setEnvironment(Environment env) {
    _environment = env;
  }

  static Environment get environment => _environment;

  static bool get isDevelopment => _environment == Environment.development;
  static bool get isProduction => _environment == Environment.production;
  static bool get isStaging => _environment == Environment.staging;

  static String get org {
    switch (_environment) {
      case Environment.development:
        return 'dev.tempo';
      case Environment.production:
        return 'com.tempo';
      case Environment.staging:
        return 'com.tempo.pre';
    }
  }

  static bool get showDevTools => !isProduction;
}
