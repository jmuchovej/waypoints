import java.io.FileInputStream
import java.util.Properties

plugins {
  id("com.android.application")
  id("org.jetbrains.kotlin.android")
  id("rust")
}

val tauriProperties =
    Properties().apply {
      val propFile = file("tauri.properties")
      if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
      }
    }

val waypointsFlavor = System.getenv("WAYPOINTS_FLAVOR") ?: "stable"
val waypointsBuild = System.getenv("WAYPOINTS_BUILD") == "1"

android {
  compileSdk = 36
  namespace = "so.waypoints"
  defaultConfig {
    manifestPlaceholders["usesCleartextTraffic"] = "false"
    // Only suffix the applicationId for distribution builds — during dev, the
    // Tauri CLI hardcodes the launch intent to the base identifier.
    applicationId =
        "so.waypoints" +
            if (waypointsBuild) {
              when (waypointsFlavor) {
                "development" -> ".dev"
                "preview" -> ".preview"
                "nightly" -> ".nightly"
                else -> ""
              }
            } else {
              ""
            }
    minSdk = 24
    targetSdk = 36
    // Nightlies share a base semver (e.g. 0.2.0-nightly.<date>), so the Tauri-
    // derived versionCode collides across builds and blocks in-place updates.
    // Let CI inject a monotonic code (WAYPOINTS_VERSION_CODE) for that channel;
    // stable/preview fall back to the Tauri-generated value.
    versionCode =
        (System.getenv("WAYPOINTS_VERSION_CODE")?.takeIf { it.isNotBlank() }
                ?: tauriProperties.getProperty("tauri.android.versionCode", "1"))
            .toInt()
    versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")

    resValue(
        "string",
        "app_name",
        when (waypointsFlavor) {
          "development" -> "Waypoints (Dev)"
          "preview" -> "Waypoints (Preview)"
          "nightly" -> "Waypoints (Nightly)"
          else -> "Waypoints"
        },
    )
    resValue(
        "string",
        "main_activity_title",
        when (waypointsFlavor) {
          "development" -> "Waypoints (Dev)"
          "preview" -> "Waypoints (Preview)"
          "nightly" -> "Waypoints (Nightly)"
          else -> "Waypoints"
        },
    )
  }

  signingConfigs {
    create("release") {
      val keystorePropertiesFile = rootProject.file("keystore.properties")
      val keystoreProperties = Properties()
      if (keystorePropertiesFile.exists()) {
        keystoreProperties.load(FileInputStream(keystorePropertiesFile))
        keyAlias = keystoreProperties["keyAlias"] as String
        keyPassword = keystoreProperties["password"] as String
        storeFile = file(keystoreProperties["storeFile"] as String)
        storePassword = keystoreProperties["password"] as String
      }
    }
  }

  buildTypes {
    getByName("debug") {
      manifestPlaceholders["usesCleartextTraffic"] = "true"
      isDebuggable = true
      isJniDebuggable = true
      isMinifyEnabled = false
      packaging {
        jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
        jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
        jniLibs.keepDebugSymbols.add("*/x86/*.so")
        jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
      }
    }
    getByName("release") {
      isMinifyEnabled = true
      proguardFiles(
          *fileTree(".") { include("**/*.pro") }
              .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
              .toList()
              .toTypedArray()
      )
      signingConfig = signingConfigs.getByName("release")
    }
  }
  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
  kotlinOptions { jvmTarget = "17" }
  buildFeatures { buildConfig = true }
}

rust { rootDirRel = "../../../" }

dependencies {
  implementation("androidx.webkit:webkit:1.14.0")
  implementation("androidx.appcompat:appcompat:1.7.1")
  implementation("androidx.activity:activity-ktx:1.10.1")
  implementation("com.google.android.material:material:1.12.0")
  testImplementation("junit:junit:4.13.2")
  androidTestImplementation("androidx.test.ext:junit:1.1.4")
  androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")
