import com.android.build.gradle.AppExtension

val android = project.extensions.getByType(AppExtension::class.java)

android.apply {
    flavorDimensions("environment")

    productFlavors {
        create("development") {
            dimension = "environment"
            applicationId = "com.tempo.app.dev"
            resValue(type = "string", name = "app_name", value = "Tempo (Dev)")
        }
        create("staging") {
            dimension = "environment"
            applicationId = "com.tempo.app.staging"
            resValue(type = "string", name = "app_name", value = "Tempo (Staging)")
        }
        create("production") {
            dimension = "environment"
            applicationId = "com.tempo.app.prod"
            resValue(type = "string", name = "app_name", value = "Tempo")
        }
    }
}