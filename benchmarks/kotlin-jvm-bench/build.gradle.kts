plugins {
    kotlin("jvm") version "2.3.20"
    kotlin("plugin.allopen") version "2.3.20"
    id("me.champeau.jmh") version "0.7.2"
    application
}

group = "com.example"
version = "1.0-SNAPSHOT"

val boltffiDir = "${projectDir}/../rust-boltffi/target/release"
val uniffiDir = "${projectDir}/../rust-uniffi/target/release"
val nativePath = listOf(boltffiDir, uniffiDir).joinToString(":")

repositories {
    mavenCentral()
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
    implementation("net.java.dev.jna:jna:5.17.0")
}

application {
    mainClass.set("com.example.bench_compare.CompareMainKt")
}

val buildBoltffi by tasks.registering(Exec::class) {
    workingDir = projectDir
    commandLine("../rust-boltffi/build-kotlin.sh")
}

val buildJni by tasks.registering(Exec::class) {
    dependsOn(buildBoltffi)
    workingDir = projectDir
    commandLine("./build-jni.sh")
}

val buildUniffi by tasks.registering(Exec::class) {
    workingDir = projectDir
    commandLine("../rust-uniffi/build-kotlin.sh")
}

tasks.named("compileKotlin") {
    dependsOn(buildJni)
    dependsOn(buildUniffi)
}

tasks.matching { it.name.startsWith("jmh") }.configureEach {
    dependsOn(buildJni)
    dependsOn(buildUniffi)
}

tasks.named("jmh") {
    doFirst {
        file("${layout.buildDirectory.get()}/tmp/jmh/jmh.lock").delete()
    }
}

tasks.withType<JavaExec> {
    jvmArgs(
        "-Djava.library.path=$nativePath",
        "-Djna.library.path=$nativePath",
    )
}

allOpen {
    annotation("org.openjdk.jmh.annotations.State")
}

jmh {
    jmhVersion = "1.37"
    fork = 1
    warmupIterations = 3
    iterations = 3
    warmup = "1s"
    timeOnIteration = "1s"
    resultFormat = "JSON"
    val include = providers.gradleProperty("jmhInclude").orNull
    if (include != null) {
        includes = listOf(include)
    }
    jvmArgsAppend = listOf(
        "-Djava.library.path=$nativePath",
        "-Djna.library.path=$nativePath",
    )
}

kotlin {
    sourceSets {
        named("main") {
            kotlin.srcDir("${projectDir}/../rust-boltffi/dist/android/kotlin")
            kotlin.srcDir("${projectDir}/../rust-uniffi/dist/kotlin")
        }
    }
    jvmToolchain(25)
}
