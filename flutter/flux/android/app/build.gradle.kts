plugins {
  id("com.android.application")
  id("kotlin-android")
  // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
  id("dev.flutter.flutter-gradle-plugin")
}

android {
  namespace = "com.tempo.flux"
  compileSdk = flutter.compileSdkVersion
  ndkVersion = flutter.ndkVersion

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
  }

  kotlinOptions { jvmTarget = JavaVersion.VERSION_11.toString() }

  defaultConfig {
    // TODO: Specify your own unique Application ID
    // (https://developer.android.com/studio/build/application-id.html).
    applicationId = "com.tempo.flux"
    // You can update the following values to match your application needs.
    // For more information, see: https://flutter.dev/to/review-gradle-config.
    minSdk = flutter.minSdkVersion
    targetSdk = flutter.targetSdkVersion
    versionCode = flutter.versionCode
    versionName = flutter.versionName
  }

  buildTypes {
    // getByName("debug") { }
    release {
      // TODO: Add your own signing config for the release build.
      // Signing with the debug keys for now, so `flutter run --release` works.
      signingConfig = signingConfigs.getByName("debug")
    }
  }

  flavorDimensions += "environment"
  productFlavors {
    create("development") {
      dimension = "environment"
      applicationIdSuffix = ".dev"
      resValue(type = "string", name = "app_name", value = "Tempo Flux Dev")
    }
    create("staging") {
      dimension = "environment"
      applicationIdSuffix = ".stag"
      resValue(type = "string", name = "app_name", value = "Tempo Flux Staging")
    }
    create("production") {
      dimension = "environment"
      applicationIdSuffix = ".prod"
      resValue(type = "string", name = "app_name", value = "Tempo Flux")
    }
  }
}

flutter { source = "../.." }

