plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.compose.compiler)
}

val repoDirectory = rootProject.layout.projectDirectory.dir("..")
val nativeOutput = repoDirectory.dir("target/generated")
val sudachiAssets = nativeOutput.dir("sudachi-assets")
val dictionaryAssets = nativeOutput.dir("dictionary-assets")
val bundleDictionaries =
    tasks.register<Exec>("bundleDictionaries") {
        group = "build"
        description = "Verify and bundle prepared JMdict/JMnedict index, sources and notices"
        workingDir(repoDirectory)
        commandLine("python3", repoDirectory.file("tools/bundle-dictionaries.py").asFile)
        inputs.files(
            repoDirectory.file("tools/bundle-dictionaries.py"),
            repoDirectory.file("tools/prepare-dictionaries.py"),
            repoDirectory.file("third_party/dictionaries/android.lock.json"),
        )
        inputs.dir(repoDirectory.dir(".local/dictionaries/index-v1-20261025"))
        outputs.dir(dictionaryAssets)
    }
val bundleSudachi =
    tasks.register<Exec>("bundleSudachi") {
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

fun rustBridgeTask(variant: String) =
    tasks.register<Exec>("buildRustBridge${variant.replaceFirstChar { it.uppercase() }}") {
        group = "build"
        description =
            "Generate Kotlin bindings and package the Rust/JNI bridge for both Android ABIs"
        workingDir(repoDirectory)
        commandLine("bash", repoDirectory.file("tools/build-android-native.sh").asFile, variant)
        environment("ANDROID_HOME", androidComponents.sdkComponents.sdkDirectory.get().asFile)
        inputs.files(repoDirectory.file("Cargo.toml"), repoDirectory.file("Cargo.lock"))
        inputs.files(
            fileTree(repoDirectory.dir("crates")) {
                include("**/*.rs", "**/*.toml", "**/*.sql")
                exclude("**/target/**")
            }
        )
        inputs.file(repoDirectory.file("tools/build-android-native.sh"))
        inputs.file(repoDirectory.file("tools/verify-sudachi-source.py"))
        inputs.dir(repoDirectory.dir("third_party/sudachi"))
        outputs.dirs(
            nativeOutput.dir("$variant/kotlin"),
            nativeOutput.dir("$variant/native"),
            nativeOutput.dir("$variant/android"),
        )
    }

val buildRustBridgeDebug = rustBridgeTask("debug")
val buildRustBridgeRelease = rustBridgeTask("release")

buildRustBridgeRelease.configure { mustRunAfter(buildRustBridgeDebug) }

val buildSudachiAppProbe =
    tasks.register<Exec>("buildSudachiAppProbe") {
        group = "verification"
        description = "Build the measurement-only Sudachi JNI library for debug APKs"
        dependsOn(buildRustBridgeDebug) // Native builds share Cargo output; keep them sequential.
        workingDir(repoDirectory)
        commandLine("python3", repoDirectory.file("tools/build-sudachi-app-probe.py").asFile)
        environment("ANDROID_HOME", androidComponents.sdkComponents.sdkDirectory.get().asFile)
        inputs.files(repoDirectory.file("Cargo.toml"), repoDirectory.file("Cargo.lock"))
        inputs.files(
            fileTree(repoDirectory.dir("crates")) { include("**/*.rs", "**/*.toml", "**/*.sql") }
        )
        inputs.dir(repoDirectory.dir("third_party/sudachi"))
        inputs.file(repoDirectory.file("tools/build-sudachi-app-probe.py"))
        inputs.file(repoDirectory.file("tools/verify-sudachi-source.py"))
        outputs.dir(nativeOutput.dir("sudachi-probe/jniLibs"))
    }

android {
    namespace = "app.yomoku"
    compileSdk = 37
    buildToolsVersion = "36.0.0"
    // Must match crates/yomoku-ffi/boltffi.toml; also used by AGP's native tooling.
    ndkVersion = "30.0.16248370"

    defaultConfig {
        applicationId = "app.yomoku"
        minSdk = 29
        targetSdk = 37
        versionCode = 2
        versionName = "1.0.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    buildTypes {
        release {
            optimization { enable = true }
        }
    }

    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "x86_64")
            isUniversalApk = true
        }
    }

    sourceSets.named("main") {
        assets.directories += sudachiAssets.asFile.absolutePath
        assets.directories += dictionaryAssets.asFile.absolutePath
    }
    listOf("debug", "release").forEach { variant ->
        sourceSets.named(variant) {
            kotlin.directories +=
                listOf(
                    nativeOutput.dir("$variant/kotlin/app").asFile.absolutePath,
                    nativeOutput.dir("$variant/kotlin/com").asFile.absolutePath,
                    nativeOutput.dir("$variant/native/app").asFile.absolutePath,
                )
            jniLibs.directories += nativeOutput.dir("$variant/android/jniLibs").asFile.absolutePath
        }
    }
    sourceSets.named("debug") {
        jniLibs.directories += nativeOutput.dir("sudachi-probe/jniLibs").asFile.absolutePath
    }
    sourceSets.named("androidTest") {
        kotlin.directories += repoDirectory.dir("tests/shared/kotlin").asFile.absolutePath
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

androidComponents {
    onVariants(selector().withBuildType("debug")) { variant ->
        variant.outputs.forEach { output ->
            if (output.filters.isEmpty()) {
                output.outputFileName.set("app-debug.apk")
            } else {
                output.enabled.set(false)
            }
        }
    }
    onVariants(selector().withBuildType("release")) { variant ->
        variant.outputs.forEach { output ->
            val abi = output.filters.singleOrNull()?.identifier ?: "universal"
            if (abi == "x86_64") {
                output.enabled.set(false)
            } else {
                output.outputFileName.set(output.versionName.map { "yomoku-$it-$abi-unsigned.apk" })
            }
        }
    }
}

tasks.named("preBuild") { dependsOn(bundleSudachi, bundleDictionaries) }

tasks.matching { it.name == "preDebugBuild" }.configureEach { dependsOn(buildSudachiAppProbe) }

tasks.matching { it.name == "preReleaseBuild" }.configureEach { dependsOn(buildRustBridgeRelease) }

buildSudachiAppProbe.configure { mustRunAfter(buildRustBridgeRelease) }

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors = true
    }
}

dependencies {
    implementation(libs.androidsvg)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.viewmodel)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui.tooling.preview)
    debugImplementation(libs.androidx.compose.ui.tooling)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.espresso.core)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
}
