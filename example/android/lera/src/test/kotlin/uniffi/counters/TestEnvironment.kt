package uniffi.counters

import timber.log.Timber
import java.nio.file.Files
import java.nio.file.Paths
import java.util.concurrent.atomic.AtomicBoolean

internal object TestEnvironment {
    private val timberInstalled = AtomicBoolean(false)

    init {
        configureNativeLibrary()
        installTimber()
    }

    fun ensure() = Unit

    private fun configureNativeLibrary() {
        val osName = System.getProperty("os.name")?.lowercase() ?: ""
        val libraryExtension = when {
            osName.contains("mac") -> "dylib"
            osName.contains("win") -> "dll"
            else -> "so"
        }

        val targetRoot = Paths.get("..", "..", "rust", "target").toAbsolutePath().normalize()
        val candidate = Files.walk(targetRoot).use { paths ->
            paths
                .filter { Files.isRegularFile(it) && it.fileName.toString() == "libcounters.$libraryExtension" }
                .findFirst()
                .orElseThrow {
                    IllegalStateException("Unable to locate libcounters.$libraryExtension under $targetRoot")
                }
        }

        System.setProperty("uniffi.component.counters.libraryOverride", candidate.toString())
    }

    private fun installTimber() {
        if (timberInstalled.compareAndSet(false, true)) {
            Timber.plant(object : Timber.Tree() {
                override fun log(priority: Int, tag: String?, message: String, t: Throwable?) {
                    val output = buildString {
                        if (tag != null) append(tag).append(": ")
                        append(message)
                        if (t != null) {
                            appendLine()
                            append(t.stackTraceToString())
                        }
                    }
                    println(output)
                }
            })
        }
    }
}
