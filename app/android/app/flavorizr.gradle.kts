import com.android.build.gradle.AppExtension

val android = project.extensions.getByType(AppExtension::class.java)

android.apply {
  flavorDimensions("environment")

  productFlavors {
    create("development") {
      dimension = "environment"
      applicationId = "com.waypoints.app.dev"
      resValue(type = "string", name = "app_name", value = "Waypoints (Dev)")
    }
    create("staging") {
      dimension = "environment"
      applicationId = "com.waypoints.app.staging"
      resValue(type = "string", name = "app_name", value = "Waypoints (Staging)")
    }
    create("production") {
      dimension = "environment"
      applicationId = "com.waypoints.app.prod"
      resValue(type = "string", name = "app_name", value = "Waypoints")
    }
  }
}
