import java.io.File

plugins {
    java
    id("me.champeau.jmh") version "0.7.3"
}

group = "com.example"
version = "1.0-SNAPSHOT"

val uniffiDir = "${projectDir}/../../adapters/uniffi/target/release"
val boltffiJvmDir = "${projectDir}/../../generated/boltffi/dist/java"
val nativePath = listOf(uniffiDir, boltffiJvmDir).joinToString(File.pathSeparator)

repositories {
    mavenCentral()
}

val buildUniffiJava by tasks.registering(Exec::class) {
    workingDir = projectDir
    commandLine("../../adapters/uniffi/build-java.sh")
}

val buildBoltffiJava by tasks.registering(Exec::class) {
    workingDir = projectDir
    commandLine("../../generated/boltffi/build-java.sh")
}

tasks.named("compileJava") {
    dependsOn(buildUniffiJava)
    dependsOn(buildBoltffiJava)
}

tasks.matching { it.name.startsWith("jmh") }.configureEach {
    dependsOn(buildUniffiJava)
    dependsOn(buildBoltffiJava)
}

tasks.named("jmh") {
    doFirst {
        file("${layout.buildDirectory.get()}/tmp/jmh/jmh.lock").delete()
    }
}

tasks.withType<JavaExec> {
    jvmArgs(
        "-Djava.library.path=$nativePath",
        "--enable-native-access=ALL-UNNAMED",
    )
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
        "--enable-native-access=ALL-UNNAMED",
    )
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(25)
    }
    sourceSets {
        named("main") {
            java.srcDir("${projectDir}/../../adapters/uniffi/dist/java")
            java.srcDir("${projectDir}/../../generated/boltffi/dist/java")
        }
    }
}
