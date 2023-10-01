import com.github.jengelman.gradle.plugins.shadow.tasks.ShadowJar
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    id("com.github.johnrengelman.shadow") version "8.1.1"
    kotlin("jvm") version "1.9.10"
    kotlin("plugin.serialization") version "1.9.10"
}

val javaVersion = JavaVersion.VERSION_1_8
val javaVersionNumber = javaVersion.name.substringAfter('_').replace('_', '.')
val javaVersionMajor = javaVersion.name.substringAfterLast('_')

val r8: Configuration by configurations.creating
dependencies {
    implementation(kotlin("serialization"))
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-hocon:1.6.0")
    r8("com.android.tools:r8:+")
}


tasks.withType<KotlinCompile> {
    kotlinOptions.jvmTarget = javaVersionNumber
}

tasks.withType<AbstractCompile> {
    sourceCompatibility = javaVersionNumber
    targetCompatibility = javaVersionNumber
}

tasks.withType<Jar> {
    manifest {
        attributes(mapOf("Main-Class" to "MainKt"))
    }
}

tasks.withType<ShadowJar> {
    archiveClassifier.set("shadow")
    mergeServiceFiles()
    minimize()
    exclude("**/*.kotlin_*")
}

tasks {
    assemble {
        dependsOn(shadowJarMinified)
    }
}

val shadowJarMinified = tasks.register<JavaExec>("shadowJarMinified") {
    dependsOn(configurations.runtimeClasspath)
    
    val proguardRules = file("src/main/proguard-rules.pro")
    inputs.files(tasks.shadowJar.get().outputs.files, proguardRules)
    
    val r8File = layout.buildDirectory.file("libs/${base.archivesName.get()}-shadow-minified.jar").get().asFile
    outputs.file(r8File)
    
    classpath(r8)
    
    mainClass.set("com.android.tools.r8.R8")
    val javaHome = File(ProcessHandle.current().info().command().get()).parentFile.parentFile.canonicalPath
    val args = mutableListOf(
        //"--debug",
        "--classfile",
        "--output",
        r8File.toString(),
        "--pg-conf",
        proguardRules.toString(),
        "--lib",
        javaHome,
    )
    args.add(tasks.shadowJar.get().outputs.files.joinToString(" "))
    
    this.args = args
    
    doFirst {
        val javaHomeVersion = Runtime.version()
        check(JavaVersion.toVersion(javaHomeVersion).isCompatibleWith(javaVersion)) {
            "Incompatible Java Versions: compile-target $javaVersionNumber, r8 runtime $javaHomeVersion (needs to be as new or newer)"
        }
        
        check(proguardRules.exists()) { "$proguardRules doesn't exist" }
    }
}
