import java.io.File
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

val minSdkVersion = run {
  val config = rootProject.file("../../configs/platforms/android.toml")
  val declared =
      Regex("""^\s*minSdkVersion\s*=\s*(\d+)""", RegexOption.MULTILINE)
          .find(config.readText())
          ?.groupValues
          ?.get(1)
  requireNotNull(declared) { "minSdkVersion is not declared in ${config.path}" }.toInt()
}

android {
  compileSdk = 36
  buildToolsVersion = libs.versions.buildTools.get()
  namespace = "so.waypoints"
  defaultConfig {
    manifestPlaceholders["usesCleartextTraffic"] = "false"
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
    minSdk = minSdkVersion
    targetSdk = 36
    versionCode =
        (System.getenv("WAYPOINTS_BUILD_CODE")?.takeIf { it.isNotBlank() }
                ?: tauriProperties.getProperty("tauri.android.versionCode", "1"))
            .toInt()
    // Rendered the way TestFlight renders iOS's pair of fields — `0.1.0
    // (2026.10.10)` — so the two stores show the same thing. The prerelease
    // is dropped: the flavor already says which channel this is.
    versionName =
        System.getenv("WAYPOINTS_VERSION")
            ?.takeIf { it.isNotBlank() }
            ?.let { version ->
              val release = version.substringBefore('-').substringBefore('+')
              val build = version.substringAfter('+', "")
              if (build.isEmpty()) release else "$release ($build)"
            } ?: tauriProperties.getProperty("tauri.android.versionName", "1.0")

    resValue(
        "string",
        "app_name",
        when (waypointsFlavor) {
          "development" -> "Waypoints Development Edition"
          "preview" -> "Waypoints Preview"
          "nightly" -> "Waypoints Nightly"
          else -> "Waypoints"
        },
    )
    resValue(
        "string",
        "main_activity_title",
        when (waypointsFlavor) {
          "development" -> "Waypoints Development Edition"
          "preview" -> "Waypoints Preview"
          "nightly" -> "Waypoints Nightly"
          else -> "Waypoints"
        },
    )
  }

  signingConfigs {
    create("release") {
      val keystorePath =
          requireNotNull(System.getenv("ANDROID_KEYSTORE_PATH")) {
            "ANDROID_KEYSTORE_PATH is unset — run the build through " +
                "`just tauri::build android`, which wraps it in `secretspec run`."
          }
      val keystorePropertiesFile = File(keystorePath)
      require(keystorePropertiesFile.exists()) {
        "ANDROID_KEYSTORE_PATH points at $keystorePath, which does not exist."
      }

      val keystoreProperties = Properties()
      keystoreProperties.load(FileInputStream(keystorePropertiesFile))
      keyAlias = keystoreProperties["keyAlias"] as String
      keyPassword = keystoreProperties["password"] as String
      storeFile = file(keystoreProperties["storeFile"] as String)
      storePassword = keystoreProperties["password"] as String
    }
  }

  buildTypes {
    getByName("debug") {
      applicationIdSuffix = ".debug"
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
  sourceSets {
    // The launcher icon is per flavor — `src/<flavor>/res` holds its adaptive
    // icon and layers — and the Waypoints flavor is not a Gradle product
    // flavor (those are Tauri's ABI ones), so nothing would include that
    // directory unless it is named here. It rides on the build types rather
    // than `main`: the flavor directories also carry copies of `main`'s
    // values, and a build type's resources override `main`'s while a second
    // directory on `main` itself would collide with them.
    for (type in listOf("debug", "release")) {
      getByName(type) { res.srcDir("src/$waypointsFlavor/res") }
    }
  }
  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
  kotlin { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) } }
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
