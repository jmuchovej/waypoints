import com.android.build.gradle.AppExtension

val android = project.extensions.getByType(AppExtension::class.java)

android.apply {
    flavorDimensions("environment")

    productFlavors {
        create("development") {
            dimension = "environment"
            applicationId = "com.tempo.flux.dev"
            resValue(type = "string", name = "app_name", value = "Tempo Flux (Dev)")
        }
        create("staging") {
            dimension = "environment"
            applicationId = "com.tempo.flux.staging"
            resValue(type = "string", name = "app_name", value = "Tempo Flux (Staging)")
        }
        create("production") {
            dimension = "environment"
            applicationId = "com.tempo.flux.prod"
            resValue(type = "string", name = "app_name", value = "Tempo Flux")
        }
    }
}