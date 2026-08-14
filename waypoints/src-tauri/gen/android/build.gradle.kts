buildscript {
  repositories {
    google()
    mavenCentral()
  }
  dependencies {
    classpath("com.android.tools.build:gradle:8.13.2")
    classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.0.21")
  }
}

allprojects {
  repositories {
    google()
    mavenCentral()
  }
}

// `tauri.settings.gradle` includes tauri-android and every tauri-plugin-* as
// real subprojects. These each hardcode `JavaVersion.VERSION_1_8` and
// `jvmTarget = "1.8"`, which JDK 21 reports as obsolete on every build:
//
//     Java compiler version 21 has deprecated support for compiling with
//     source/target version 8.
//     warning: [options] source value 8 is obsolete ...
//
// So, raise the level after each subproject's script body runs. Raised levels
// are linked, so both `jvmTarget` and `JavaVersion` must be raised together.
subprojects {
  afterEvaluate {
    extensions.findByType<com.android.build.gradle.LibraryExtension>()?.apply {
      compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
      }
    }
  }

  tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
    compilerOptions {
      jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
  }
}

tasks.register("clean").configure { delete("build") }
