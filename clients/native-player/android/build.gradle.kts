plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

repositories {
    google()
    mavenCentral()
}

android {
    namespace = "com.taitidelabs.hg.player"
    compileSdk = 35

    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    implementation("androidx.media3:media3-exoplayer:1.5.1")
    implementation("androidx.media3:media3-exoplayer-hls:1.5.1")

    val localAar = file("libs/nsh.aar")
    val localJar = file("libs/nsh-classes.jar")
    val externalSdk = providers.gradleProperty("nsh.sdkJar").orNull
        ?: providers.environmentVariable("NSH_ANDROID_SDK_JAR").orNull

    when {
        localAar.exists() -> compileOnly(files(localAar))
        localJar.exists() -> compileOnly(files(localJar))
        !externalSdk.isNullOrBlank() -> compileOnly(files(externalSdk))
    }

    testImplementation("junit:junit:4.13.2")
}
