plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.compose.compiler)
}

val repoDirectory = rootProject.layout.projectDirectory.dir("..")
val nativeOutput = repoDirectory.dir("target/generated")
val sudachiAssets = nativeOutput.dir("sudachi-assets")
val bundleSudachi by
    tasks.registering(Exec::class) {
        group = "build"
        description =
            "Verify and bundle the explicitly prepared Sudachi Full dictionary and notices"
        workingDir(repoDirectory)
        commandLine(
            "python3",
            repoDirectory.file("tools/prepare-sudachi.py").asFile,
            "--android-assets",
        )
        inputs.file(repoDirectory.file("tools/prepare-sudachi.py"))
        inputs.dir(repoDirectory.dir(".local/sudachi/full-20261015"))
        outputs.dir(sudachiAssets)
    }
val buildRustBridge by
    tasks.registering(Exec::class) {
        group = "build"
        description =
            "Generate Kotlin bindings and package the Rust/JNI bridge for both Android ABIs"
        workingDir(repoDirectory)
        commandLine("bash", repoDirectory.file("tools/build-android-native.sh").asFile)
        environment("ANDROID_HOME", androidComponents.sdkComponents.sdkDirectory.get().asFile)
        inputs.files(repoDirectory.file("Cargo.toml"), repoDirectory.file("Cargo.lock"))
        inputs.files(
            fileTree(repoDirectory.dir("crates")) {
                include("**/*.rs", "**/*.toml")
                exclude("**/target/**")
            }
        )
        inputs.file(repoDirectory.file("tools/build-android-native.sh"))
        outputs.dirs(
            nativeOutput.dir("kotlin"),
            nativeOutput.dir("native"),
            nativeOutput.dir("android"),
        )
    }

val buildSudachiAppProbe by
    tasks.registering(Exec::class) {
        group = "verification"
        description = "Build the measurement-only Sudachi JNI library for debug APKs"
        dependsOn(buildRustBridge) // Native builds share Cargo output; keep them sequential.
        workingDir(repoDirectory)
        commandLine("python3", repoDirectory.file("tools/build-sudachi-app-probe.py").asFile)
        environment("ANDROID_HOME", androidComponents.sdkComponents.sdkDirectory.get().asFile)
        inputs.files(repoDirectory.file("Cargo.toml"), repoDirectory.file("Cargo.lock"))
        inputs.files(fileTree(repoDirectory.dir("crates")) { include("**/*.rs", "**/*.toml") })
        inputs.dir(repoDirectory.dir("third_party/sudachi"))
        inputs.file(repoDirectory.file("tools/build-sudachi-app-probe.py"))
        inputs.file(repoDirectory.file("tools/verify-sudachi-source.py"))
        outputs.dir(nativeOutput.dir("sudachi-probe/jniLibs"))
    }

android {
    namespace = "app.yomoku"
    compileSdk = 36
    buildToolsVersion = "36.0.0"
    // Must match crates/yomoku-ffi/boltffi.toml; also used by AGP's native tooling.
    ndkVersion = "30.0.16248370"

    defaultConfig {
        applicationId = "app.yomoku"
        minSdk = 29
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    sourceSets.named("main") {
        assets.directories += sudachiAssets.asFile.absolutePath
        kotlin.directories +=
            listOf(
                nativeOutput.dir("kotlin/app").asFile.absolutePath,
                nativeOutput.dir("kotlin/com").asFile.absolutePath,
                nativeOutput.dir("native/app").asFile.absolutePath,
            )
        jniLibs.directories += nativeOutput.dir("android/jniLibs").asFile.absolutePath
    }
    sourceSets.named("debug") {
        jniLibs.directories += nativeOutput.dir("sudachi-probe/jniLibs").asFile.absolutePath
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
        aidl = false
        buildConfig = false
        shaders = false
    }
    lint {
        warningsAsErrors = true
        // Versions are deliberately pinned and reviewed separately.
        disable += "NewerVersionAvailable"
        disable += "AndroidGradlePluginVersion"
        disable += "GradleDependency"
    }
}

tasks.named("preBuild") { dependsOn(buildRustBridge, bundleSudachi) }

tasks.matching { it.name == "preDebugBuild" }.configureEach { dependsOn(buildSudachiAppProbe) }

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors = true
    }
}

dependencies {
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.viewmodel)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui.tooling.preview)
    debugImplementation(libs.androidx.compose.ui.tooling)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
}
