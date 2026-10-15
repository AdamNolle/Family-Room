plugins {
    id("com.android.application") version "8.10.1" apply false
    id("org.jetbrains.kotlin.android") version "2.2.20" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.2.20" apply false
}

// File Provider may duplicate generated classes in a synced checkout. Keep
// compiler outputs in a local cache, with isolation between checkouts.
val nativeBuildRoot = System.getenv("FAMILY_ROOM_ANDROID_BUILD_DIR")?.let(::File)
    ?: File(System.getProperty("user.home"), ".cache/family-room/android/${rootDir.absolutePath.hashCode().toUInt().toString(16)}")
allprojects {
    layout.buildDirectory.set(File(nativeBuildRoot, if (path == ":") "root" else path.trim(':').replace(':', '/')))
}
tasks.register("printAppBuildDirectory") {
    doLast { println(project(":app").layout.buildDirectory.get().asFile.absolutePath) }
}
